use crate::models::{Category, Server};
use askama_actix::Template;

#[derive(Template, Debug)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub(crate) categories: Vec<Category>,
    pub(crate) current_category: Option<Category>,
}

#[derive(Template, Debug)]
#[template(path = "details.html")]
pub struct DetailsTemplate {
    pub(crate) server: Server,
}

#[derive(Template, Debug)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {}
