use reqwest::{Response};
use serde_json::Value;
use dotenv;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let rpc_client_id = dotenv::var("DISCORD_APPLICATION_ID").unwrap_or_else(|_| "".to_string());
    let url = dotenv::var("JELLYFIN_URL").unwrap_or_else(|_| "".to_string());
    let api_key = dotenv::var("JELLYFIN_API_KEY").unwrap_or_else(|_| "".to_string());
    let username = dotenv::var("JELLYFIN_USERNAME").unwrap_or_else(|_| "".to_string());
    
    let mut connected: bool = false;
    let mut start_time: i64 = 0;
    let mut drpc = DiscordIpcClient::new(rpc_client_id.as_str()).expect("Failed to create Discord RPC client, discord is down or the Client ID is invalid.");
    let img: String = "https://s2.qwant.com/thumbr/0x380/f/1/a63bf84e940773357439bba0cd9544a5626f172fe1e65f6fc873818cda4103/uybguvnj1p821.png?u=https%3A%2F%2Fi.redd.it%2Fuybguvnj1p821.png".to_string();
    let mut curr_details: String = "".to_string();
    // Start loop
    loop {
        let jfresult = match get_jellyfin_playing(&url, &api_key, &username).await {
            Ok(res) => res,
            Err(_) => vec!["".to_string()],
        };
        let media_type = &jfresult[0];

        if media_type != "" {
            let mut state_message: String = "".to_owned();
            let mut details: String = "".to_owned();
            if media_type == "episode" {
                details = "Watching ".to_owned() + &jfresult[1][1..jfresult[1].len() - 1];
                state_message = "S".to_owned() + jfresult[3].as_str() + "E" + jfresult[4].as_str() + " " + &jfresult[2][1..jfresult[2].len() - 1];
            } else if media_type == "movie" {
                details = format!("{}", jfresult[1]);
            }
            if connected != true {
                // Start up the client connection, so that we can actually send and receive stuff
                loop {
                    match drpc.connect() {
                        Ok(result) => result,
                        Err(_) => {
                            println!("Failed to connect, retrying in 10 seconds"); 
                            std::thread::sleep(std::time::Duration::from_secs(10)); 
                            continue
                        },
                    };
                    break;
                }
                println!("//////////////////////////////////////////////////////////////////\nConnected to Discord RPC client\n//////////////////////////////////////////////////////////////////\n{}", details);

                // Set the starting time for the timestamp
                start_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
                // Set current state message
                curr_details = details.to_owned();
                // Set connected to true so that we don't try to connect again
                connected = true;
            } else if details != curr_details {
                    // Disconnect from the client
                drpc.close().expect("Failed to close Discord RPC client");
                std::thread::sleep(std::time::Duration::from_secs(8));
                // Set connected to false so that we dont try to disconnect again
                connected = false;
                println!("Disconnected from Discord RPC client");
                std::thread::sleep(std::time::Duration::from_secs(18));
                continue;
            }
            // Set the activity
            if media_type == "episode" {
                drpc.set_activity(
                    activity::Activity::new()
                    // Set the "state" or message
                    .state(&state_message)
                    .details(&details)
                    // Add a timestamp
                    .timestamps(activity::Timestamps::new()
                        .start(start_time)
                    )
                    // Add image and a link to the github repo
                    .assets(
                        activity::Assets::new()
                            .large_image(&img)
                            .large_text("https://github.com/Radiicall/jellyfin-rpc") 
                    )
                ).expect("Failed to set activity");   
            } else if media_type == "movie" {
                drpc.set_activity(
                    activity::Activity::new()
                    // Set the "state" or message
                    .state(&details)
                    // Add a timestamp
                    .timestamps(activity::Timestamps::new()
                        .start(start_time)
                    )
                    // Add image and a link to the github repo
                    .assets(
                        activity::Assets::new()
                            .large_image(&img)
                            .large_text("https://github.com/Radiicall/jellyfin-rpc") 
                    )
                ).expect("Failed to set activity");   
            }
        } else if connected == true {
            // Disconnect from the client
            drpc.close().expect("Failed to close Discord RPC client");
            std::thread::sleep(std::time::Duration::from_secs(8));
            // Set connected to false so that we dont try to disconnect again
            connected = false;
            println!("Disconnected from Discord RPC client");
        }
    // Sleep for 18 seconds
    std::thread::sleep(std::time::Duration::from_secs(18));
    }
}

async fn get_jellyfin_playing(url: &String, api_key: &String, username: &String) -> Result<Vec<String>, reqwest::Error> {
    // Create the request
    let url = format!("{}/Sessions?api_key={}", url, api_key);
    // Get response
    let res: Response = reqwest::get(url).await?;
    
    // Get the body of the response
    let body = res.text().await?;
    
    // Convert to json
    let json: Vec<Value> = serde_json::from_str(&body).unwrap();
    let mut name: String;
    let mut series_name: String;
    let mut season: String;
    let mut episode: String;
    let mut itemtype: String;
    for i in 0..json.len() {
        match json[i].get("UserName") {
            None => continue,
            _ => (),
        };
        if json[i].get("UserName").unwrap().as_str().unwrap() == username {
            match json[i].get("NowPlayingItem") {
                None => continue,
                _ => (),
            };
            let nowplayingitem = json[i].get("NowPlayingItem").expect("Couldn't find NowPlayingItem.");
            name = nowplayingitem.get("Name").expect("Couldn't find Name").to_string();
            if nowplayingitem.get("Type").unwrap().as_str().unwrap() == "Episode" {
                itemtype = "episode".to_owned();
                series_name = nowplayingitem.get("SeriesName").expect("Couldn't find SeriesName.").to_string();
                season = nowplayingitem.get("ParentIndexNumber").expect("Couldn't find IndexNumber.").to_string();
                episode = nowplayingitem.get("IndexNumber").expect("Couldn't find IndexNumber.").to_string();

                if name != "" {
                    let result: Vec<String> = vec![itemtype, series_name, name, season, episode];
                    return Ok(result);
                }
            } else if nowplayingitem.get("Type").unwrap().as_str().unwrap() == "Movie" {
                itemtype = "movie".to_owned();

                if name != "" {
                    let result: Vec<String> = vec![itemtype, name];
                    return Ok(result);
                }
            }
        }
    }
    Ok(vec!["".to_owned()])
}
