use std::io;

use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

fn main() -> io::Result<()> {
    // TODO: Get values from a config file
    let paste_dir = String::from("./pastes");
    let url_base = String::from("https://localhost");
    let config = Config {
        paste_dir,
        url_base,
    };
    let config = web::Data::new(config);

    HttpServer::new(move || App::new().register_data(config.clone()).service(new_paste))
        .bind("127.0.0.1:8080")
        .unwrap()
        .run()
}

#[derive(Clone)]
struct Config {
    pub paste_dir: String,
    pub url_base: String,
    // TODO: Fields for HTTP auth (user/pass)
}

#[derive(Deserialize)]
struct Paste {
    pub data: String,
}

#[post("/")]
fn new_paste(config: web::Data<Config>, paste: web::Form<Paste>) -> impl Responder {
    // TODO: Generate the paste name randomly
    // TODO: Consider using multipart formdata instead of urlencoded.
    let paste_name = "pastename";
    let file_name = format!("{}/{}.txt", config.paste_dir, paste_name);

    println!("{}", paste.data);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("{}/{}", config.url_base, paste_name))
}
