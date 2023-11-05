#[macro_use] extern crate rocket;

use std::collections::HashMap;
use std::sync::{Arc,RwLock};
use rocket::State;
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::response::content::RawHtml;
use chrono::{DateTime, Utc};

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
    jointime: i64,
    pfp: Option<String>,
}

#[get("/")]
fn root(listholder: &State<PlayerListHolder>) -> RawHtml<String>{
    let list = listholder.playerlist.read().unwrap();
    let mut output = "<!DOCTYPE html>\n<html>\n<head>\n<style>\nbody {background-color: black;}\nh1, h2 {color: blue;}\ntable {\n border-collapse: collapse;\n width: 40%;\n}\n\nth, td {\n border-left:2px solid MidnightBlue;\n border-right:2px solid MidnightBlue;\n border-bottom: 2px solid MidnightBlue;\n text-align: center;\n}\n</style>\n</head>\n<body>\n<h1 style=\"text-align: center; color: blue;\">Headless server users</h1>\n<table style=\"margin-left: auto; margin-right: auto;\">\n<tbody>\n<tr>\n<td><h2>Profile Picture</h2></td>\n<td><h2>Username</h2></td>\n<td><h2>Time since join</h2></td>\n</tr>\n".to_string();
    for player in list.list.iter() {
        output += "<tr>\n";
        //Boykisser as default pfp lol
        output += &format!("<td><img src=\"{}\" width=\"64\" height=\"64\" /></td>\n",player.pfp.as_ref().unwrap_or(&"https://i.imgur.com/Zl1DcHg.png".to_string()));
        output += &format!("<td><h1>{}</h1></td>\n",player.username);
        let now = Utc::now();
        let joindt: DateTime<Utc> = DateTime::from_timestamp(player.jointime,0u32).expect("timestamp fail????");
        let duration = (now - joindt).to_std().expect("duration conversion");
        let seconds = duration.as_secs() % 60;
        let minutes = (duration.as_secs() / 60) % 60;
        let hours = (duration.as_secs() / 60) / 60;
        output += &format!("<td><h2 class=\"duration\" data-timejoined={}>{}:{}:{}</h2></td>\n",player.jointime,hours,minutes,seconds);
        output += "</tr>\n";
    }
    output += "</tbody>\n</table>\n<script>\nconst collection = document.getElementsByClassName(\"duration\");\n\nsetInterval(function () {\nfor (let i = 0; i < collection.length; i++) {\n  let time = collection[i].dataset.timejoined;\n  let duration = Math.floor((Date.now() - (time*1000))/1000);\n  let seconds = duration % 60;\n  let minutes = Math.floor(duration / 60) % 60;\n  let hours = Math.floor(Math.floor(duration / 60) / 60);\n  let output = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;\n  collection[i].innerHTML = output;\n}\n}, 1000);\n\nsetTimeout(function(){\n   window.location.reload();\n}, 60000);\n</script>\n</body>\n</html>";
    return RawHtml(output);
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
        let getpfp = reqwest::get(format!("https://api.resonite.com/users/{}",i)).await.unwrap().json::<UserResponse>().await;
        match getpfp {
            Ok(pfp) => {
                let mut mapwrite = pfpmap.map.write().unwrap();
                let assetid = pfp.profile.iconurl.split("resdb:///").last().expect("known format").split(".").next().expect("known format");
                let asseturl = format!("https://assets.resonite.com/{}",assetid);
                mapwrite.insert(i.to_string(),asseturl);
            }
            Err(_) => {
                continue;
            }
        }
        
    }

    let mut list = listholder.playerlist.write().unwrap();
    let map = pfpmap.map.read().unwrap();
    for i in newlist.list.iter_mut() {
        i.pfp = map.get(&i.userid).cloned();
    }
    *list = newlist;
}

#[post("/userjoin", format = "json", data="<player>")]
async fn userjoin(player: Json<Player>, webhookurl: &State<WebhookUrl>) {
    let url = webhookurl.url.clone();
    let message = DiscordWebhookMessage::newjoin(&player.username);
    let client = reqwest::Client::new();
    let _res = client.post(&*url)
    .json(&message)
    .send()
    .await.unwrap();
}

#[post("/userleave", format = "json", data="<player>")]
async fn userleave(player: Json<Player>, webhookurl: &State<WebhookUrl>) {
    let url = webhookurl.url.clone();
    let message = DiscordWebhookMessage::newleave(&player.username);
    let client = reqwest::Client::new();
    let _res = client.post(&*url)
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
    .mount("/", routes![root])
    .mount("/", routes![list])
    .mount("/", routes![userjoin])
    .mount("/", routes![userleave])
    .manage(PlayerListHolder { playerlist: RwLock::new(PlayerList {list: Vec::new()})})
    .manage(PlayerPfpMap { map: RwLock::new(HashMap::<String,String>::new())})
    .manage(WebhookUrl { url: Arc::new(webhookurl)})
}