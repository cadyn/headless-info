#[macro_use] extern crate rocket;

use std::collections::HashMap;
use std::sync::{Arc,RwLock};
use rocket::State;
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::fairing::AdHoc;

struct WebhookUrl {
    url: Arc<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct DiscordWebhookMessage {
    content: String,
    tts: bool,
    embeds: Vec<DiscordEmbed>,
}

impl DiscordWebhookMessage {
    fn newjoin(username: &str) -> Self {
        DiscordWebhookMessage { content: "".to_string(), tts: false, embeds: vec![DiscordEmbed{
            id: 652627557,
            title: format!("{} joined the headless.",username),
            color: 65280,
        }] }
    }

    fn newleave(username: &str) -> Self {
        DiscordWebhookMessage { content: "".to_string(), tts: false, embeds: vec![DiscordEmbed{
            id: 862528582,
            title: format!("{} left the headless.",username),
            color: 16711680,
        }] }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct DiscordEmbed {
    id: isize,
    title: String,
    color: isize,
}

struct PlayerPfpMap {
    map: RwLock<HashMap<String,String>>
}
struct PlayerListHolder {
    playerlist: RwLock<PlayerList>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct UserResponse {
    profile: Profile
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct Profile {
    #[serde(rename = "iconUrl")]
    iconurl: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[serde(transparent)]
struct PlayerList {
    list: Vec<Player>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct Player {
    username: String,
    userid: String,
    jointime: u64,
    pfp: Option<String>,
}

#[get("/list")]
fn list(listholder: &State<PlayerListHolder>) -> Json<PlayerList>{
    let list = listholder.playerlist.read().unwrap();
    Json((*list).clone())
}

#[post("/update", format = "json", data = "<data>")]
async fn update(data: Json<PlayerList>, listholder: &State<PlayerListHolder>, pfpmap: &State<PlayerPfpMap>) {
    //let mut list = listholder.playerlist.write().unwrap();
    
    let mut newlist = (*data).clone();
    let mut toupdate: Vec<&str> = Vec::new();
    {
        let map = pfpmap.map.read().unwrap();
        for i in newlist.list.iter_mut() {
            if !map.contains_key(&i.userid) {
                toupdate.push(&i.userid);
            }
        }
    }
    for i in toupdate {
        let getpfp = reqwest::get(format!("https://api.resonite.com/users/{}",i)).await.unwrap().json::<UserResponse>().await.unwrap();
        let mut mapwrite = pfpmap.map.write().unwrap();
        let assetid = getpfp.profile.iconurl.split("resdb:///").last().expect("known format").split(".").next().expect("known format");
        let asseturl = format!("https://assets.resonite.com/{}",assetid);
        mapwrite.insert(i.to_string(),asseturl);
    }

    let mut list = listholder.playerlist.write().unwrap();
    let map = pfpmap.map.read().unwrap();
    for i in newlist.list.iter_mut() {
        i.pfp = Some(map[&i.userid].clone());
    }
    *list = newlist;
}

#[post("/userjoin", format = "json", data="<player>")]
async fn userjoin(player: Json<Player>, webhookurl: &State<WebhookUrl>) {
    let url = (webhookurl.url.clone());
    let message = DiscordWebhookMessage::newjoin(&player.username);
    let client = reqwest::Client::new();
    let res = client.post(&*url)
    .json(&message)
    .send()
    .await.unwrap();
}

#[post("/userleave", format = "json", data="<player>")]
async fn userleave(player: Json<Player>, webhookurl: &State<WebhookUrl>) {
    let url = (webhookurl.url.clone());
    let message = DiscordWebhookMessage::newleave(&player.username);
    let client = reqwest::Client::new();
    let res = client.post(&*url)
    .json(&message)
    .send()
    .await.unwrap();
}

#[launch]
fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();

    let webhookurl: String = figment.extract_inner("webhook").expect("webhook");

    rocket.mount("/", routes![update])
    .mount("/", routes![list])
    .mount("/", routes![userjoin])
    .mount("/", routes![userleave])
    .manage(PlayerListHolder { playerlist: RwLock::new(PlayerList {list: Vec::new()})})
    .manage(PlayerPfpMap { map: RwLock::new(HashMap::<String,String>::new())})
    .manage(WebhookUrl { url: Arc::new(webhookurl)})
}