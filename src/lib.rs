use actix_multipart::Multipart;
use actix_web::dev::ServiceRequest;
use actix_web::{get, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::{future, Future};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::iter;
use syntect::highlighting::{Color, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let config = web::Data::new(config);
    // Realm is hardcoded for now. I will consider getting it from the config file.
    let auth_config = web::Data::new(
        actix_web_httpauth::extractors::basic::Config::default().realm("rustpaste pastebin"),
    );
    let form = form_data::Form::new().field("paste", form_data::Field::text());

    HttpServer::new(move || {
        let basic_auth = HttpAuthentication::basic(authenticate);
        App::new()
            .data(form.clone())
            .register_data(config.clone())
            .register_data(auth_config.clone())
            .service(
                web::resource("/")
                    .route(web::post().to_async(new_paste))
                    .wrap(basic_auth),
            )
            .service(send_paste)
            .service(send_highlighted_paste)
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run()?;

    Ok(())
}

#[derive(Clone)]
pub struct Config {
    pub paste_dir: String,
    pub url_base: String,
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
        // TODO: Get values from a config file
        // TODO: Parse command line arguments
        let paste_dir = String::from("./pastes");
        let url_base = String::from("https://localhost");
        let username = String::from("tansly");
        let password = String::from("hebele");
        Ok(Config {
            paste_dir,
            url_base,
            username,
            password,
        })
    }
}

// XXX: This will hang in an infinite loop if the paste directory does not exist.
// We'll probably make sure it exists while parsing config, not here.
fn new_paste(
    config: web::Data<Config>,
    (mp, form): (Multipart, web::Data<form_data::Form>),
) -> impl Future<Item = HttpResponse, Error = form_data::Error> {
    form_data::handle_multipart(mp, form.get_ref().clone()).map(move |form_value| {
        // XXX: Can we safely unwrap form_value, thus avoiding this check?
        let paste = match form_value.text() {
            Some(paste) => paste,
            None => return HttpResponse::InternalServerError().into(),
        };

        let mut rng = thread_rng();

        // Paste IDs (= paste file names) are 8 character alphanumeric strings.
        // Here we generate a random ID that is not already in use,
        // and create (and open) a paste file with that ID as its name.
        let (mut file, paste_id) = loop {
            let id: String = iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(8)
                .collect();
            let full_path = format!("{}/{}", config.paste_dir, id);
            if let Ok(file) = OpenOptions::new().write(true).create(true).open(full_path) {
                break (file, id);
            }
        };

        let paste_url = format!("{}/{}", config.url_base, paste_id);

        match file.write_all(paste.as_bytes()) {
            Ok(_) => HttpResponse::Created()
                .set_header("Location", paste_url.clone())
                .content_type("text/plain")
                .body(paste_url),
            Err(_) => HttpResponse::InternalServerError().into(),
        }
    })
}

#[get("/{paste_id}")]
fn send_paste(
    config: web::Data<Config>,
    paste_id: web::Path<String>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    web::block(move || {
        let file_path = format!("{}/{}", config.paste_dir, paste_id);
        fs::read_to_string(file_path)
    })
    .then(|res| match res {
        Ok(contents) => Ok(HttpResponse::Ok().content_type("text/plain").body(contents)),
        Err(_) => Ok(HttpResponse::NotFound().into()),
    })
}

#[get("/{paste_id}/{file_extension}")]
fn send_highlighted_paste(
    config: web::Data<Config>,
    paste_info: web::Path<(String, String)>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    web::block(move || {
        let (ref paste_id, ref file_extension) = *paste_info;

        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();

        let style = "
            pre {
                font-size:13px;
                font-family: Consolas, \"Liberation Mono\", Menlo, Courier, monospace;
            }";
        let mut body = format!(
            "<head><title>{}</title><style>{}</style></head>",
            paste_id, style
        );

        let theme = &ts.themes["base16-ocean.dark"];
        let color = theme.settings.background.unwrap_or(Color::WHITE);
        body.push_str(&format!(
            "<body style=\"background-color:#{:02x}{:02x}{:02x};\">\n",
            color.r, color.g, color.b
        ));

        let file_path = format!("{}/{}", config.paste_dir, paste_id);
        fs::read_to_string(file_path).and_then(|contents| {
            match ss.find_syntax_by_extension(file_extension) {
                Some(syntax) => {
                    let highlighted = highlighted_html_for_string(&contents, &ss, syntax, theme);
                    Ok(body + &highlighted + "</body>")
                }
                None => Ok(contents),
            }
        })
    })
    .then(|res| match res {
        Ok(contents) => Ok(HttpResponse::Ok().content_type("text/html").body(contents)),
        Err(_) => Ok(HttpResponse::NotFound().into()),
    })
}

fn authenticate(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> impl Future<Item = ServiceRequest, Error = actix_web::Error> {
    use actix_web_httpauth::extractors;
    // I wish I could chain these if let bindings.
    // https://github.com/rust-lang/rust/issues/53667 :(
    if let Some(config) = req.app_data::<Config>() {
        if let Some(password) = credentials.password() {
            let username = credentials.user_id();
            // By the way, plaintext password :))))
            if username == &config.username && password == &config.password {
                return future::ok(req);
            }
        }
    }
    // Fail safe by erroring if app configuration could not be acquired.
    let auth_config = req
        .app_data::<extractors::basic::Config>()
        .map_or_else(extractors::basic::Config::default, |data| {
            data.get_ref().clone()
        });
    future::err(extractors::AuthenticationError::from(auth_config).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http, test};
    use base64;
    use std::fs::File;
    use std::io::Write;
    use std::str;
    use tempfile::TempDir;

    fn make_test_config(paste_dir: &str) -> Config {
        Config {
            paste_dir: String::from(paste_dir),
            url_base: String::from("https://testurl"),
            username: String::from("tansly"),
            password: String::from("hebele"),
        }
    }

    #[test]
    fn get_paste_short_text() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());

        let data = web::Data::new(config.clone());
        let mut app = test::init_service(App::new().register_data(data).service(send_paste));

        // Write a test paste
        let paste_id = "/testpaste";
        let paste_content = b"plain text paste contents\nwith a newline";
        let mut file = File::create(config.paste_dir + paste_id).unwrap();
        file.write_all(paste_content).unwrap();

        let req = test::TestRequest::get().uri(paste_id).to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");

        let resp_body = test::read_body(resp);
        assert_eq!(paste_content[..], resp_body);
    }

    #[test]
    fn get_paste_nonexistent() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());
        let data = web::Data::new(config.clone());
        let mut app = test::init_service(App::new().register_data(data).service(send_paste));
        let non_existent_paste = "/hebele";

        let req = test::TestRequest::get()
            .uri(non_existent_paste)
            .to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn post_paste_short_text() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());

        let data = web::Data::new(config.clone());
        let mut app = test::init_service(
            App::new()
                .register_data(data)
                .route("/", web::post().to_async(new_paste)),
        );

        // XXX: This is not urlencoded, but it seems to work. Why?
        let paste_content = "hebele hubele\nbubele mubele\n";

        let req = test::TestRequest::post()
            .header("content-type", "application/x-www-form-urlencoded")
            .set_payload(format!("data={}", paste_content))
            .to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::CREATED);

        // Check the URL in Location header.
        let paste_url = resp.headers().get("Location").unwrap().to_str().unwrap();
        assert!(paste_url.starts_with(&config.url_base));

        let (_, paste_id) = paste_url.split_at(config.url_base.len());
        // Line above gets the paste id with a preceding slash, which is required for the next line to work.
        let file_content = fs::read_to_string(config.paste_dir + paste_id).unwrap();
        assert_eq!(paste_content, file_content);

        // Check the URL in the body.
        let resp_body = test::read_body(resp);
        let paste_url = str::from_utf8(&resp_body).unwrap();
        assert!(paste_url.starts_with(&config.url_base));
    }

    #[test]
    fn auth_valid_creds() {
        let config = make_test_config("unused path");
        let data = web::Data::new(config.clone());
        let basic_auth = HttpAuthentication::basic(authenticate);
        let mut app = test::init_service(
            App::new().register_data(data).service(
                web::resource("/")
                    .route(web::post().to(|| HttpResponse::Ok()))
                    .wrap(basic_auth),
            ),
        );

        let creds = config.username + ":" + &config.password;
        let creds = base64::encode(&creds);
        let req = test::TestRequest::post()
            .header("Authorization", format!("Basic {}", creds))
            .to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[test]
    #[should_panic]
    fn auth_invalid_user_password() {
        let config = make_test_config("unused path");
        let data = web::Data::new(config.clone());
        let basic_auth = HttpAuthentication::basic(authenticate);
        let mut app = test::init_service(
            App::new().register_data(data).service(
                web::resource("/")
                    .route(web::post().to(|| HttpResponse::Ok()))
                    .wrap(basic_auth),
            ),
        );

        let creds = config.username + "wrong_user:wrong_password" + &config.password;
        let creds = base64::encode(&creds);
        let req = test::TestRequest::post()
            .header("Authorization", format!("Basic {}", creds))
            .to_request();
        // test::call_service() panics if the result is an error, if I understood correctly.
        // I'd like to get a response and check the status code, but well.
        test::call_service(&mut app, req);
    }

    #[test]
    #[should_panic]
    fn auth_invalid_user() {
        let config = make_test_config("unused path");
        let data = web::Data::new(config.clone());
        let basic_auth = HttpAuthentication::basic(authenticate);
        let mut app = test::init_service(
            App::new().register_data(data).service(
                web::resource("/")
                    .route(web::post().to(|| HttpResponse::Ok()))
                    .wrap(basic_auth),
            ),
        );

        let creds = config.username + "wrong_user:" + &config.password;
        let creds = base64::encode(&creds);
        let req = test::TestRequest::post()
            .header("Authorization", format!("Basic {}", creds))
            .to_request();
        // test::call_service() panics if the result is an error, if I understood correctly.
        // I'd like to get a response and check the status code, but well.
        test::call_service(&mut app, req);
    }

    #[test]
    #[should_panic]
    fn auth_invalid_password() {
        let config = make_test_config("unused path");
        let data = web::Data::new(config.clone());
        let basic_auth = HttpAuthentication::basic(authenticate);
        let mut app = test::init_service(
            App::new().register_data(data).service(
                web::resource("/")
                    .route(web::post().to(|| HttpResponse::Ok()))
                    .wrap(basic_auth),
            ),
        );

        let creds = config.username + ":wrong_password" + &config.password;
        let creds = base64::encode(&creds);
        let req = test::TestRequest::post()
            .header("Authorization", format!("Basic {}", creds))
            .to_request();
        // test::call_service() panics if the result is an error, if I understood correctly.
        // I'd like to get a response and check the status code, but well.
        test::call_service(&mut app, req);
    }

    #[test]
    #[should_panic]
    fn auth_no_header() {
        let config = make_test_config("unused path");
        let data = web::Data::new(config.clone());
        let basic_auth = HttpAuthentication::basic(authenticate);
        let mut app = test::init_service(
            App::new().register_data(data).service(
                web::resource("/")
                    .route(web::post().to(|| HttpResponse::Ok()))
                    .wrap(basic_auth),
            ),
        );

        let req = test::TestRequest::post().to_request();
        // test::call_service() panics if the result is an error, if I understood correctly.
        // I'd like to get a response and check the status code, but well.
        test::call_service(&mut app, req);
    }
}
