use std::error::Error;
use std::fs::File;
use std::io::{self, ErrorKind, Read};
use url::Url;
use serde_yaml;
use serde_json::{Value};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ModList{
    mods: Vec<Mod>,
}

#[derive(Debug, Deserialize)]
struct Mod{
    name: String,
    url: String,
    installed_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Release{
    name: String,
    url: String,
    installed_version: Option<String>,
}

fn main() {
    // Specify the path to your YAML file
    let mods_file_path = "./mods.yaml";

    // Attempt to read the YAML file
    match read_file(mods_file_path) {
        Ok(mod_list) => {
            for mod_item in mod_list.mods {
                // Parse the GitHub repository URL
                if let Ok(url) = Url::parse(&mod_item.url) {
                    if url.host_str() == Some("github.com") {
                        let path_segments: Vec<_> = url.path_segments().unwrap().collect();
                        println!("{:?}", path_segments);
                        if path_segments.len() >= 2 {
                            let owner = path_segments[0];
                            let repo = path_segments[1];
                            download_latest_release(owner, repo, &mod_item.name).expect("didint");
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

fn read_file(file_path: &str) -> Result<ModList, Box<dyn Error>> {
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


fn download_latest_release(owner: &str, repo: &str, mod_name: &str) -> Result<(), Box<dyn Error>> {
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
    
    // Check if "assets" array is present and has at least one element
    if let Some(assets) = parsed_json["assets"].as_array() {
        if let Some(download_url) = assets[0]["browser_download_url"].as_str() {
            println!("Download URL: {}", download_url);
                // Make a GET request to the file URL
            let mut response = reqwest::blocking::get(download_url)?;

            // Ensure the request was successful
            // response.error_for_status()?;

            // Create a local file to save the downloaded content
            let mut file = File::create(format!("{}_latest.zip", mod_name))?;

            // Write the content to the local file
            response.copy_to(&mut file)?;

            println!("File downloaded successfully.");

        } else {
            println!("Download URL not found in the JSON response.");
        }
    } else {
        println!("No assets found in the JSON response.");
    }

    Ok(())
}