use std::error::Error;
use std::fs::File;
use std::io::{self, ErrorKind, Read, prelude::*};
use url::Url;
use serde_yaml;
use serde_json::{Value};
use serde::Deserialize;
use std::io::Cursor;
use std::path::Path;
use zip;

#[derive(Debug, Deserialize)]
struct ModList{
    install_dir: String,
    mods: Vec<Mod>,
}

#[derive(Debug, Deserialize)]
struct Mod{
    name: String,
    url: String,
    installed_version: Option<String>,
    create_mod_dir: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Release{
    name: String,
    url: String,
    installed_version: Option<String>,
}

fn main() {

    println!("Mod Manager: Staring");

    // Specify the path to your YAML file
    let mods_file_path = "./mods.yaml";

    println!("Mod Manager: Reading mods from file -> {}", mods_file_path);

    // Attempt to read the YAML file
    match get_mod_list(mods_file_path) {
        Ok(mod_list) => {
            
            for mod_item in mod_list.mods {
                let mut install_dir = mod_list.install_dir.clone() ;
                // Parse the GitHub repository URL
                if let Ok(url) = Url::parse(&mod_item.url) {
                    if url.host_str() == Some("github.com") {
                        let path_segments: Vec<_> = url.path_segments().unwrap().collect();
                        if path_segments.len() >= 2 {
                            let owner = path_segments[0];
                            let repo = path_segments[1];
                            println!("Mod Manager: Downloading mod from GitHub -> {}", &mod_item.name);
                            let mut mod_file_name = download_release_from_github(owner, repo, &mod_item.name).expect("Mod Manager: Download Failed");
                            println!("Mod Manager: Extracting & Installing mod -> {}", &mod_item.name);
                            if mod_item.create_mod_dir.is_some() {
                                install_dir = format!("{}/{}", install_dir, mod_item.name.to_string());
                            }
                            extract_file(mod_item.name.to_string(), mod_file_name, install_dir.clone()).expect("Extract error");
                        }
                    }
                }
            }
        }
        Err(e) => {
            if let Some(io_error) = e.downcast_ref::<io::Error>() {
                if io_error.kind() == ErrorKind::NotFound {
                    eprintln!("Error: File not found: {}", mods_file_path);
                } else {
                    eprintln!("Error: Failed to read or parse configuration file: {}", e);
                }
            } else {
                eprintln!("Error: Failed to read or parse configuration file: {}", e);
            }
        }
    }
}

fn get_mod_list(file_path: &str) -> Result<ModList, Box<dyn Error>> {
    // Open the file
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => return Err(Box::new(e) as Box<dyn Error>), // Missing file
            _ => return Err(Box::new(e) as Box<dyn Error>), // Other I/O errors
        },
    };

   // Create a buffered reader to read the file contents
   let mut reader = io::BufReader::new(file);

   // Read the contents of the file into a String
   let mut file_contents = String::new();
   reader.read_to_string(&mut file_contents)?;

   // Parse the YAML string into a vector of Mod structs
   let result: Result<ModList, serde_yaml::Error> = serde_yaml::from_str(&file_contents);

   match result {
       Ok(mods) => Ok(mods),
       Err(e) => Err(Box::new(e) as Box<dyn Error>), // Parse error
   }
}


fn download_release_from_thunderstore() {

}

fn download_release_from_github(owner: &str, repo: &str, mod_name: &str) -> Result<String, Box<dyn Error>> {
    // Construct the GitHub releases API URL
    let api_url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);

    // Create a reqwest client with a custom User-Agent header
    let client = reqwest::blocking::Client::builder()
        .user_agent("ModManger/1.0")
        .build()?;

    // Make a GET request to the GitHub API with the custom User-Agent header
    let response = client.get(&api_url).send()?;
    
    // Ensure the request was successful
    // response.error_for_status()?;

    // Parse the JSON response
    let parsed_json: Value = response.json()?;

    let mut mod_file_name = String::from("");
    
    // Check if "assets" array is present and has at least one element
    if let Some(assets) = parsed_json["assets"].as_array() {
        if let Some(download_url) = assets[0]["browser_download_url"].as_str() {
            // println!("Download URL: {}", download_url);
                // Make a GET request to the file URL
            let mut response = reqwest::blocking::get(download_url)?;

            // Ensure the request was successful
            // response.error_for_status()?;

            let mut tag_name = parsed_json["tag_name"].as_str().unwrap();

            mod_file_name = format!("{}_{}.zip", mod_name, tag_name);

            // Create a local file to save the downloaded content
            let mut file = File::create(format!("./mod_cache/{}_{}.zip", mod_name, tag_name))?;

            // Write the content to the local file
            response.copy_to(&mut file)?;

            println!("Mod Manager: Mod downloaded successfully.");

        } else {
            println!("Download URL not found in the JSON response.");
        }
    } else {
        println!("No assets found in the JSON response.");
    }

    Ok(mod_file_name)
}


fn extract_file(mod_name: String, mod_file_name: String, install_dir: String) -> Result<(), Box<dyn std::error::Error>> {
    // Read the ZIP file into a byte buffer

    let mut full_file_path = format!("./mod_cache/{}", mod_file_name);

    println!("Mod Manager: Extracting {}", full_file_path);

        // Read the ZIP file into a byte buffer
    let mut zip_file = File::open(full_file_path)?;
    let mut zip_buffer = Vec::new();
        zip_file.read_to_end(&mut zip_buffer)?;
    
        // Create a cursor to the byte buffer
        let cursor = io::Cursor::new(zip_buffer);
    
        // Open the ZIP archive
        let mut archive = zip::ZipArchive::new(cursor)?;
    
        // Iterate over each file in the ZIP archive
        for i in 0..archive.len() {
            // Get the file entry
            let mut file = archive.by_index(i)?;

            let mut output_dir = format!("{}", install_dir);
    
            // Construct the output file path
            let output_file_path = {
                let output_path = Path::new(output_dir.as_str());
                let entry_path = file.sanitized_name();
                output_path.join(entry_path)
            };
    
            // Create directories if needed
            if let Some(parent_dir) = output_file_path.parent() {
                std::fs::create_dir_all(parent_dir)?;
            }
    
            // Extract the file content
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
    
            // Write the content to the output file
            let mut output_file = File::create(output_file_path)?;
            output_file.write_all(&content)?;
        }

        println!("Mod Manager: Mod installed successfully.");
    
        Ok(())
}