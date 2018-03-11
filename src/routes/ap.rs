use itertools::Itertools;
use rocket::Route;
use rocket::response::Content;
use rocket::http::ContentType;
use rocket_contrib::Json;

use db;
use db::models::Account;
use error::Perhaps;
use activitypub::{ActivityGuard, ActivityStreams, AsActivityPub};
use {BASE_URL, DOMAIN};

pub fn routes() -> Vec<Route> {
    routes![ap_user_object, webfinger_get_resource, webfinger_host_meta]
}

/// Returns a user as an ActivityPub object.
#[get("/users/<username>", rank = 2)]
pub fn ap_user_object(
    username: String,
    _ag: ActivityGuard,
    db_conn: db::Connection,
) -> Perhaps<ActivityStreams> {
    Ok(Account::fetch_local_by_username(&db_conn, username)?.map(|acct| acct.as_activitypub()))
}

/// A type representing the parameters of a WebFinger query.
#[derive(FromForm, Debug)]
pub struct WFQuery {
    resource: String,
}

/// Returns JRD replies to `acct:` webfinger queries; required for Mastodon to resolve our accounts.
#[get("/.well-known/webfinger?<query>")]
pub fn webfinger_get_resource(query: WFQuery, db_conn: db::Connection) -> Perhaps<Content<Json>> {
    // TODO: don't unwrap
    let (_, addr) = query
        .resource
        .split_at(query.resource.rfind("acct:").unwrap() + "acct:".len());
    let (username, domain) = addr.split('@').collect_tuple().unwrap();

    // If the webfinger address had a different domain, 404 out.
    if domain != DOMAIN.as_str() {
        return Ok(None);
    }

    let account = try_resopt!(Account::fetch_local_by_username(&db_conn, username));

    let wf_doc = json!({
        "aliases": [account.get_uri()],
        "links": [
            {
                "href": account.get_uri(),
                "rel": "http://webfinger.net/rel/profile-page",
                "type": "text/html",
            },
            {
                "href": account.get_uri(),
                "rel": "self",
                "type": "application/activity+json",
            },
        ],
        "subject": query.resource,
    });

    let wf_content = ContentType::new("application", "jrd+json");

    Ok(Some(Content(wf_content, Json(wf_doc))))
}

/// Returns metadata about well-known routes as XRD; necessary to be Webfinger-compliant.
#[get("/.well-known/host-meta")]
pub fn webfinger_host_meta() -> Content<String> {
    let xrd_xml = ContentType::new("application", "xrd+xml");

    Content(xrd_xml, format!(r#"<?xml version="1.0"?>
<XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0">
  <Link rel="lrdd" type="application/xrd+xml" template="{base}/.well-known/webfinger?resource={{uri}}"/>
</XRD>"#, base=BASE_URL.as_str()))
}
