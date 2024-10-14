
#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{get, launch, routes};
use rocket_dyn_templates::Template;
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;

fn verify_env_keys(key: &str) -> () {
    match env::var(key) {
        Ok(value) => println!("{}: {}", key, value),
        Err(e) => eprintln!("Coldn't read Variable: {}", key)
    }
}


#[launch]
fn rocket() -> _ {
    dotenv().ok();

    // Verify GOOGLE_API_KEY
    verify_env_keys("GOOGLE_API_KEY");
    // Verify CHANNEL_ID
    verify_env_keys("CHANNEL_ID");
    // Verify FILEPATH_DATABASE
    verify_env_keys("FILEPATH_DATABASE");
    // Verify ENVIRONMENT
    verify_env_keys("ENVIRONMENT");

    rocket::build()
        .mount("/", routes![index])
        .attach(Template::fairing())
}

#[get("/")]
fn index() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("index", &context)
}

