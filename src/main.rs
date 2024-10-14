
#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{get, launch, routes};
use rocket_dyn_templates::Template;
use std::collections::HashMap;

#[launch]
fn rocket() -> _ {
   rocket::build()
       .mount("/", routes![index])
       .attach(Template::fairing())
}

#[get("/")]
fn index() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("index", &context)
}

