use std::{sync::{RwLock, Arc}, collections::{HashMap, HashSet}};

use actix_web::{web, HttpResponse, CustomizeResponder, get, http::StatusCode, post};
use alea_js::Alea;
use anyhow::anyhow;
use dearrow_parser::{StringSet, Title, TitleFlags, Thumbnail, ThumbnailFlags};
use serde::{Deserialize, Serialize};

use crate::{utils::{self, IfNoneMatch}, state::DatabaseState, etag_shortcircuit, etagged_json};

type CustomizedJsonResult<T> = utils::Result<CustomizeResponder<web::Json<T>>>;
type DBLock = web::Data<RwLock<DatabaseState>>;
type StringSetLock = web::Data<RwLock<StringSet>>;

pub fn configure_disabled(cfg: &mut web::ServiceConfig) {
    cfg.default_service(web::to(disabled_route));
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_video_branding)
        .service(post_video_branding)
        .service(get_chunk_branding)
        .service(get_user_info)
        .default_service(web::to(unknown_route));

}

async fn disabled_route() -> HttpResponse {
    HttpResponse::NotFound().body("SponsorBlockServer emulation is disabled on this DeArrow Browser instance.")
}

async fn unknown_route() -> HttpResponse {
    HttpResponse::NotFound().body("Unknown or unimplemented endpoint. Only /api/branding and /api/userInfo are implemented. See the README.md of DeArrow Browser to learn more about the limitations of SponsorBlockServer emulation.")
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct VideoBrandingParams {
    videoID: String,
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    returnUserID: bool,
    #[serde(default)]
    fetchAll: bool,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiTitle {
    title: String,
    original: bool,
    votes: i8,
    locked: bool,
    UUID: Arc<str>,
    #[serde(skip_serializing_if="Option::is_none")]
    userID: Option<Arc<str>>,
}

impl SBApiTitle {
    fn from_db(title: &Title, include_userid: bool) -> SBApiTitle {
        SBApiTitle {
            // https://github.com/ajayyy/SponsorBlockServer/blob/af31f511a53a7e30ad27123656a911393200672b/src/routes/getBranding.ts#L58
            title: title.title.replace('<', "‹"),
            original: title.flags.contains(TitleFlags::Original),
            votes: title.votes.saturating_sub(title.downvotes).saturating_sub(if title.flags.contains(TitleFlags::Unverified){ 1 } else { 0 }),
            locked: title.flags.contains(TitleFlags::Locked),
            UUID: title.uuid.clone(),
            userID: include_userid.then(|| title.user_id.clone()),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiThumbnail {
    timestamp: Option<f64>,
    original: bool,
    votes: i8,
    locked: bool,
    UUID: Arc<str>,
    #[serde(skip_serializing_if="Option::is_none")]
    userID: Option<Arc<str>>,
}

impl SBApiThumbnail {
    fn from_db(thumb: &Thumbnail, include_userid: bool) -> SBApiThumbnail {
        SBApiThumbnail {
            timestamp: thumb.timestamp,
            original: thumb.flags.contains(ThumbnailFlags::Original),
            votes: thumb.votes.saturating_sub(thumb.downvotes),
            locked: thumb.flags.contains(ThumbnailFlags::Locked),
            UUID: thumb.uuid.clone(),
            userID: include_userid.then(|| thumb.user_id.clone()),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug, Clone)]
struct SBApiVideo {
    titles: Vec<SBApiTitle>,
    thumbnails: Vec<SBApiThumbnail>,
    randomTime: f64,
    videoDuration: Option<f64>,
}

// https://github.com/ajayyy/SponsorBlockServer/blob/af31f511a53a7e30ad27123656a911393200672b/src/routes/getBranding.ts#L233
fn get_random_time_for_video(video_id: &str) -> f64 {
    let random_time = Alea::new(video_id).random();

    if random_time > 0.9 {
        random_time - 0.9
    } else {
        random_time
    }
}

fn unknown_video(video_id: &str) -> SBApiVideo {
    SBApiVideo {
        titles: vec![],
        thumbnails: vec![],
        randomTime: get_random_time_for_video(video_id),
        videoDuration: None,
    }
}

#[post("/api/branding")]
async fn post_video_branding() -> HttpResponse {
    HttpResponse::NotFound().body("Voting through DeArrow Browser is not supported. See the README.md of DeArrow Browser to learn more about the limitations of SponsorBlockServer emulation.")
}

#[get("/api/branding")]
async fn get_video_branding(db_lock: DBLock, string_set: StringSetLock, query: web::Query<VideoBrandingParams>, inm: IfNoneMatch) -> CustomizedJsonResult<SBApiVideo> {
    etag_shortcircuit!(db_lock, inm);
    let video_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(query.0.videoID.as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    if let Some(service) = query.0.service {
        if service != "YouTube" {
            return Ok(etagged_json!(db, unknown_video(&query.0.videoID)).with_status(StatusCode::NOT_FOUND));
        }
    }
    match video_id {
        None => Ok(etagged_json!(db, unknown_video(&query.0.videoID)).with_status(StatusCode::NOT_FOUND)),
        Some(video_id) => {
            Ok(etagged_json!(db, SBApiVideo {
                titles: {
                    let mut titles: Vec<SBApiTitle> = db.db.titles.iter()
                        .filter(|t| 
                            Arc::ptr_eq(&t.video_id, &video_id) 
                            && t.votes > -1 
                            && t.votes.saturating_sub(t.downvotes) > -2 
                            && !t.flags.intersects(TitleFlags::Removed | TitleFlags::ShadowHidden)
                        )
                        .map(|t| SBApiTitle::from_db(t, query.0.returnUserID))
                        .filter(|t| query.0.fetchAll || t.votes >= 0 || t.locked)
                        .collect();
                    titles.sort_unstable_by(|a, b| a.locked.cmp(&b.locked).then(a.votes.cmp(&b.votes)).reverse());
                    titles
                },
                thumbnails: {
                    let mut thumbs: Vec<SBApiThumbnail> = db.db.thumbnails.iter()
                        .filter(|t| 
                            Arc::ptr_eq(&t.video_id, &video_id) 
                            && t.votes > -1 
                            && t.votes.saturating_sub(t.downvotes) > -2 
                            && !t.flags.intersects(ThumbnailFlags::Removed | ThumbnailFlags::ShadowHidden)
                        )
                        .map(|t| SBApiThumbnail::from_db(t, query.0.returnUserID))
                        .filter(|t| query.0.fetchAll || t.votes >= 0 || t.locked)
                        .collect();
                    thumbs.sort_unstable_by(|a, b| a.locked.cmp(&b.locked).then(a.votes.cmp(&b.votes).then(a.original.cmp(&b.original).reverse())).reverse());
                    thumbs
                },
                randomTime: get_random_time_for_video(&video_id),
                videoDuration: None, // TODO: implement this
            }))
        }
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct ChunkBrandingParams {
    #[serde(default)]
    service: Option<String>,
    #[serde(default)]
    returnUserID: bool,
    #[serde(default)]
    fetchAll: bool,
}

#[derive(Deserialize, Debug)]
struct ChunkBrandingPath {
    hash_prefix: String,
}

#[get("/api/branding/{hash_prefix}")]
async fn get_chunk_branding(db_lock: DBLock, query: web::Query<ChunkBrandingParams>, path: web::Path<ChunkBrandingPath>, inm: IfNoneMatch) -> CustomizedJsonResult<HashMap<Arc<str>, SBApiVideo>> {
    etag_shortcircuit!(db_lock, inm);
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    if let Some(service) = query.0.service {
        if service != "YouTube" {
            return Ok(etagged_json!(db, HashMap::new()).with_status(StatusCode::NOT_FOUND));
        }
    }
    // validate & parse hashprefix
    if path.hash_prefix.len() != 4 {
        return Err(utils::Error::from(anyhow!("Unsupported hashprefix! Only 4-character prefixes are supported by DeArrow Browser's SponsorBlockServer emulation, but your was {} chars long!", path.hash_prefix.len())).set_status(StatusCode::BAD_REQUEST));
    }
    let hash_prefix = match u16::from_str_radix(&path.hash_prefix, 16) {
        Ok(n) => n,
        Err(_) => return Err(utils::Error::from(anyhow!("Invalid hashprefix!")).set_status(StatusCode::BAD_REQUEST)),
    };

    // Find and group details
    let mut videos: HashSet<Arc<str>> = HashSet::new();
    let mut titles: HashMap<Arc<str>, Vec<SBApiTitle>> = HashMap::new();
    db.db.titles.iter()
        .filter(|t|
            t.hash_prefix == hash_prefix
            && t.votes > -1 
            && !t.flags.intersects(TitleFlags::Removed | TitleFlags::ShadowHidden)
            && t.votes.saturating_sub(t.downvotes) > -2 
            && (query.0.fetchAll || t.flags.contains(TitleFlags::Locked) || t.votes.saturating_sub(t.downvotes) >= if t.flags.contains(TitleFlags::Unverified) { 1 } else { 0 })
        )
        .for_each(|t| match titles.get_mut(&t.video_id) {
            Some(v) => v.push(SBApiTitle::from_db(t, query.0.returnUserID)),
            None => {
                titles.insert(t.video_id.clone(), vec![SBApiTitle::from_db(t, query.0.returnUserID)]);
                videos.insert(t.video_id.clone());
            },
        });
    let mut thumbnails: HashMap<Arc<str>, Vec<SBApiThumbnail>> = HashMap::new();
    db.db.thumbnails.iter()
        .filter(|t|
            t.hash_prefix == hash_prefix
            && t.votes > -1 
            && !t.flags.intersects(ThumbnailFlags::Removed | ThumbnailFlags::ShadowHidden)
            && t.votes.saturating_sub(t.downvotes) >= if query.0.fetchAll || t.flags.contains(ThumbnailFlags::Locked) { -1 } else { 0 } 
        )
        .for_each(|t| match thumbnails.get_mut(&t.video_id) {
            Some(v) => v.push(SBApiThumbnail::from_db(t, query.0.returnUserID)),
            None => {
                thumbnails.insert(t.video_id.clone(), vec![SBApiThumbnail::from_db(t, query.0.returnUserID)]);
                videos.insert(t.video_id.clone());
            },
        });

    // Construct response
    Ok(etagged_json!(db, videos.into_iter().map(|v| (v.clone(), SBApiVideo {
        titles: titles.get(&v).cloned().unwrap_or_default(),
        thumbnails: thumbnails.get(&v).cloned().unwrap_or_default(),
        randomTime: get_random_time_for_video(&v),
        videoDuration: None // TODO: implement this
    })).collect::<HashMap<Arc<str>, SBApiVideo>>()))
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct UserInfoParams {
    publicUserID: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
struct UserInfo {
    userID: Arc<str>,
    userName: Arc<str>,
    titleSubmissionCount: usize,
    thumbnailSubmissionCount: usize,
    vip: bool,
}

#[get("/api/userInfo")]
async fn get_user_info(db_lock: DBLock, string_set: StringSetLock, query: web::Query<UserInfoParams>, inm: IfNoneMatch) -> CustomizedJsonResult<UserInfo> {
    etag_shortcircuit!(db_lock, inm);
    let user_id = string_set.read().map_err(|_| anyhow!("Failed to acquire StringSet for reading"))?
        .set.get(query.0.publicUserID.as_str()).cloned();
    let db = db_lock.read().map_err(|_| anyhow!("Failed to acquire DatabaseState for reading"))?;
    Ok(etagged_json!(db, match user_id {
        None => {
            let user_id: Arc<str> = query.0.publicUserID.into();
            UserInfo { userID: user_id.clone(), userName: user_id, titleSubmissionCount: 0, thumbnailSubmissionCount: 0, vip: false }
        },
        Some(user_id) => UserInfo {
            userName: db.db.usernames.get(&user_id).map(|u| u.username.clone()).unwrap_or_else(|| user_id.clone()),
            titleSubmissionCount: db.db.titles.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id) && t.votes >= 0).count(),
            thumbnailSubmissionCount: db.db.thumbnails.iter().filter(|t| Arc::ptr_eq(&t.user_id, &user_id) && t.votes >= 0).count(),
            vip: db.db.vip_users.contains(&user_id),
            userID: user_id,
        },
    }))
}