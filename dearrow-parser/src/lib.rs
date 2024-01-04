use std::{sync::Arc, fmt::Display, collections::{HashSet, HashMap}, path::{Path, PathBuf}, fs::File};
use enumflags2::{bitflags, BitFlags};
use anyhow::{Result, Context, Error};

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ThumbnailFlags {
    Original,
    Locked,
    ShadowHidden,
    Removed,
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TitleFlags {
    Original,
    Locked,
    ShadowHidden,
    Unverified,
    Removed,
}

#[derive(Clone)]
pub struct Thumbnail {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub downvotes: i8,
    pub flags: BitFlags<ThumbnailFlags>,
}

#[derive(Clone)]
pub struct Title {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub votes: i8,
    pub downvotes: i8,
    pub flags: BitFlags<TitleFlags>,
}

#[derive(Clone)]
pub struct Username {
    pub user_id: Arc<str>,
    pub username: Arc<str>,
    pub locked: bool,
}

#[derive(Default, Clone)]
pub struct StringSet {
    pub set: HashSet<Arc<str>>
}

impl StringSet {
    pub fn with_capacity(capacity: usize) -> StringSet {
        StringSet { set: HashSet::with_capacity(capacity) }
    }

    pub fn dedupe_struct<T: Dedupe>(&mut self, obj: &mut T) {
        obj.dedupe(self);
    }

    pub fn dedupe_arc(&mut self, reference: &mut Arc<str>) {
        if let Some(s) = self.set.get(reference) {
            *reference = s.clone();
        } else {
            self.set.insert(reference.clone());
        }
    }

    pub fn clean(&mut self) {
        self.set.retain(|s| Arc::strong_count(s) > 1)
    }
}

pub trait Dedupe {
    fn dedupe(&mut self, set: &mut StringSet);
}

impl Dedupe for Thumbnail {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
    }
}
impl Dedupe for Title {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.uuid);
        set.dedupe_arc(&mut self.title);
        set.dedupe_arc(&mut self.video_id);
        set.dedupe_arc(&mut self.user_id);
    }
}
impl Dedupe for Username {
    fn dedupe(&mut self, set: &mut StringSet) {
        set.dedupe_arc(&mut self.user_id);
        set.dedupe_arc(&mut self.username);
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    InvalidValue {
        uuid: Arc<str>,
        field: &'static str,
        value: i8,
    },
    MismatchedUUIDs {
        struct_name: &'static str,
        uuid_main: Arc<str>,
        uuid_struct: Arc<str>,
    },
    MissingSubobject {
        struct_name: &'static str,
        uuid: Arc<str>,
    }
}

#[derive(Debug)]
pub enum ObjectKind {
    Title,
    Thumbnail,
    Username,
}

#[derive(Debug)]
pub struct ParseError(ObjectKind, Box<ParseErrorKind>);

impl Display for ObjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectKind::Title => write!(f, "Title"),
            ObjectKind::Thumbnail => write!(f, "Thumbnail"),
            ObjectKind::Username => write!(f, "Username"),
        }
    }
}

impl std::error::Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let object_kind = &self.0;
        match *self.1 {
            ParseErrorKind::InvalidValue { ref uuid, ref field, ref value } => write!(f, "Parsing error: Field {field} in {object_kind} {uuid} contained an invalid value: {value}"),
            ParseErrorKind::MismatchedUUIDs { ref struct_name, ref uuid_main, ref uuid_struct } => write!(f, "Merge error: Component {struct_name} of {object_kind} {uuid_main} had a different UUID: {uuid_struct}"),
            ParseErrorKind::MissingSubobject { ref struct_name, ref uuid } => write!(f, "Parsing error: {object_kind} {uuid} was missing an associated {struct_name} object")
        }
    }
}

pub struct DearrowDB {
    pub titles: Vec<Title>,
    pub thumbnails: Vec<Thumbnail>,
    pub usernames: HashMap<Arc<str>, Username>,
    pub vip_users: HashSet<Arc<str>>,
}

pub struct DBPaths {
    pub thumbnails: PathBuf,
    pub thumbnail_timestamps: PathBuf,
    pub thumbnail_votes: PathBuf,
    pub titles: PathBuf,
    pub title_votes: PathBuf,
    pub usernames: PathBuf,
    pub vip_users: PathBuf,
}

pub type LoadResult = (DearrowDB, Vec<Error>);

impl DearrowDB {
    pub fn load_dir(dir: &Path, string_set: &mut StringSet) -> Result<LoadResult> {
        DearrowDB::load(
            DBPaths {
                thumbnails: dir.join("thumbnails.csv"),
                thumbnail_timestamps: dir.join("thumbnailTimestamps.csv"),
                thumbnail_votes: dir.join("thumbnailVotes.csv"),
                titles: dir.join("titles.csv"),
                title_votes: dir.join("titleVotes.csv"),
                usernames: dir.join("userNames.csv"),
                vip_users: dir.join("vipUsers.csv"),
            },
            string_set,
        )
    }

    pub fn load(paths: DBPaths, string_set: &mut StringSet) -> Result<LoadResult> {
        // Briefly open each file in read-only to check if they exist before continuing to parse
        drop(File::open(&paths.thumbnails).context("Could not open the thumbnails file")?);
        drop(File::open(&paths.thumbnail_timestamps).context("Could not open the thumbnail timestamps file")?);
        drop(File::open(&paths.thumbnail_votes).context("Could not open the thumbnail votes file")?);
        drop(File::open(&paths.titles).context("Could not open the titles file")?);
        drop(File::open(&paths.title_votes).context("Could not open the title votes file")?);
        drop(File::open(&paths.usernames).context("Could not open the usernames file")?);
        drop(File::open(&paths.vip_users).context("Could not open the VIP users file")?);

        // Create a vec for non-fatal deserialization errors
        let mut errors: Vec<Error> = Vec::new();
        
        // Load the entirety of thumbnailTimestamps and thumbnailVotes into HashMaps, while
        // deduplicating strings
        let thumbnail_timestamps: HashMap<Arc<str>, csv_data::ThumbnailTimestamps> = csv::Reader::from_path(&paths.thumbnail_timestamps)
            .context("Could not initialize csv reader for thumbnail timestamps")?
            .into_deserialize::<csv_data::ThumbnailTimestamps>()
            .filter_map(|result| match result.context("Error while deserializing thumbnail timestamps") {
                Ok(mut thumb) => {
                    thumb.dedupe(string_set);
                    Some(thumb)
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .map(|timestamp| (timestamp.uuid.clone(), timestamp))
            .collect();
        let thumbnail_votes: HashMap<Arc<str>, csv_data::ThumbnailVotes> = csv::Reader::from_path(&paths.thumbnail_votes)
            .context("Could not initialize csv reader for thumbnail votes")?
            .into_deserialize::<csv_data::ThumbnailVotes>()
            .filter_map(|result| match result.context("Error while deserializing thumbnail votes") {
                Ok(mut thumb) => {
                    thumb.dedupe(string_set);
                    Some(thumb)
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .map(|thumb| (thumb.uuid.clone(), thumb))
            .collect();

        // Load the Thumbnail objects while deduplicating strings and merging them with other Thumbnail* objects
        let thumbnails: Vec<Thumbnail> = csv::Reader::from_path(&paths.thumbnails)
            .context("Could not initialize csv reader for thumbnails")?
            .into_deserialize::<csv_data::Thumbnail>()
            .filter_map(|result| match result.context("Error while deserializing thumbnails") {
                Ok(mut thumb) => {
                    thumb.dedupe(string_set);
                    let timestamp = thumbnail_timestamps.get(&thumb.uuid);
                    let votes = match thumbnail_votes.get(&thumb.uuid) {
                        Some(v) => v,
                        None => {
                            errors.push(Error::new(ParseError(ObjectKind::Thumbnail, Box::new(ParseErrorKind::MissingSubobject { struct_name: "ThumbnailVotes", uuid: thumb.uuid.clone() }))));
                            return None;
                        }
                    };
                    match thumb.try_merge(timestamp, votes) {
                        Ok(t) => Some(t),
                        Err(e) => {
                            errors.push(e.into());
                            None
                        }
                    }
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .collect();

        drop(thumbnail_timestamps);
        drop(thumbnail_votes);

        // Do the same for titles
        let title_votes: HashMap<Arc<str>, csv_data::TitleVotes> = csv::Reader::from_path(&paths.title_votes)
            .context("Could not initialize csv reader for title votes")?
            .into_deserialize::<csv_data::TitleVotes>()
            .filter_map(|result| match result.context("Error while deserializing title votes") {
                Ok(mut title) => {
                    title.dedupe(string_set);
                    Some(title)
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .map(|title| (title.uuid.clone(), title))
            .collect();
        let titles: Vec<Title> = csv::Reader::from_path(&paths.titles)
            .context("Could not initialize csv reader for titles")?
            .into_deserialize::<csv_data::Title>()
            .filter_map(|result| match result.context("Error while deserializing titles") {
                Ok(mut title) => {
                    title.dedupe(string_set);
                    let votes = match title_votes.get(&title.uuid) {
                        Some(v) => v,
                        None => {
                            errors.push(Error::new(ParseError(ObjectKind::Title, Box::new(ParseErrorKind::MissingSubobject { struct_name: "TitleVotes", uuid: title.uuid.clone() }))));
                            return None;
                        }
                    };
                    match title.try_merge(votes) {
                        Ok(t) => Some(t),
                        Err(e) => {
                            errors.push(e.into());
                            None
                        }
                    }
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .collect();

        drop(title_votes);

        // Load usernames and VIP users
        let usernames: HashMap<Arc<str>, Username> = csv::Reader::from_path(&paths.usernames)
            .context("could not initialize csv reader for VIP users")?
            .into_deserialize::<csv_data::Username>()
            .filter_map(|result| match result.context("Error while deserializing titles") {
                Ok(mut username) => {
                    username.dedupe(string_set);
                    match TryInto::<Username>::try_into(username) {
                        Ok(u) => Some(u),
                        Err(e) => {
                            errors.push(e.into());
                            None
                        }
                    }
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .map(|username| (username.user_id.clone(), username))
            .collect();
        let vip_users: HashSet<Arc<str>> = csv::Reader::from_path(&paths.vip_users)
            .context("could not initialize csv reader for VIP users")?
            .into_deserialize::<csv_data::VIPUser>()
            .filter_map(|result| match result.context("Error while deserializing titles") {
                Ok(mut vip) => {
                    vip.dedupe(string_set);
                    Some(vip.user_id)
                },
                Err(error) => {
                    errors.push(error);
                    None
                }
            })
            .collect();

        Ok((DearrowDB {titles, thumbnails, usernames, vip_users}, errors))
    }
}


mod csv_data {
    use std::sync::Arc;
    use serde::Deserialize;
    use enumflags2::BitFlag;
    use super::{ParseError, ObjectKind, ParseErrorKind, ThumbnailFlags, TitleFlags, StringSet, Dedupe};

    type Result<T> = std::result::Result<T, ParseError>;

    #[derive(Deserialize)]
    pub struct Thumbnail {
        #[serde(rename="videoID")]
        video_id: Arc<str>,
        original: i8,
        #[serde(rename="userID")]
        user_id: Arc<str>,
        #[serde(rename="timeSubmitted")]
        time_submitted: i64,
        #[serde(rename="UUID")]
        pub uuid: Arc<str>,
    }

    #[derive(Deserialize)]
    pub struct ThumbnailTimestamps {
        #[serde(rename="UUID")]
        pub uuid: Arc<str>,
        timestamp: f64,
    }

    #[derive(Deserialize)]
    pub struct ThumbnailVotes {
        #[serde(rename="UUID")]
        pub uuid: Arc<str>,
        votes: i8,
        locked: i8,
        #[serde(rename="shadowHidden")]
        shadow_hidden: i8,
        downvotes: i8,
        removed: i8,
    }

    #[derive(Deserialize)]
    pub struct Title {
        #[serde(rename="videoID")]
        video_id: Arc<str>,
        title: Arc<str>,
        original: i8,
        #[serde(rename="userID")]
        user_id: Arc<str>,
        #[serde(rename="timeSubmitted")]
        time_submitted: i64,
        #[serde(rename="UUID")]
        pub uuid: Arc<str>,
    }

    #[derive(Deserialize)]
    pub struct TitleVotes {
        #[serde(rename="UUID")]
        pub uuid: Arc<str>,
        votes: i8,
        locked: i8,
        #[serde(rename="shadowHidden")]
        shadow_hidden: i8,
        verification: i8,
        downvotes: i8,
        removed: i8,
    }

    #[derive(Deserialize)]
    pub struct VIPUser {
        #[serde(rename="userID")]
        pub user_id: Arc<str>
    }

    #[derive(Deserialize)]
    pub struct Username {
        #[serde(rename="userID")]
        pub user_id: Arc<str>,
        #[serde(rename="userName")]
        pub username: Arc<str>,
        pub locked: i8,
    }

    macro_rules! intbool {
        (thumb $struct:expr, $field:ident) => {
            intbool!(! $struct, $field, ObjectKind::Thumbnail, uuid, 0, 1)
        };
        (title $struct:expr, $field:ident) => {
            intbool!(! $struct, $field, ObjectKind::Title, uuid, 0, 1)
        };
        (uname $struct:expr, $field:ident) => {
            intbool!(! $struct, $field, ObjectKind::Username, user_id, 0, 1)
        };
        (thumb $struct:expr, $field:ident, $falseint: expr, $trueint:expr) => {
            intbool!(! $struct, $field, ObjectKind::Thumbnail, uuid, $falseint, $trueint)
        };
        (title $struct:expr, $field:ident, $falseint: expr, $trueint:expr) => {
            intbool!(! $struct, $field, ObjectKind::Title, uuid, $falseint, $trueint)
        };
        (uname $struct:expr, $field:ident, $falseint: expr, $trueint:expr) => {
            intbool!(! $struct, $field, ObjectKind::Username, user_id, $falseint, $trueint)
        };
        (! $struct:expr, $field:ident, $kind:expr, $uuid:ident, $falseint:expr, $trueint:expr) => {
            match $struct.$field {
                $falseint => false,
                $trueint => true,
                _ => return Err(ParseError($kind, Box::new(ParseErrorKind::InvalidValue { uuid: $struct.$uuid.clone(), field: stringify!($field), value: $struct.$field }))),
            }
        };
    }


    impl Thumbnail {
        pub fn try_merge(self, timestamps: Option<&ThumbnailTimestamps>, votes: &ThumbnailVotes) -> Result<super::Thumbnail> {
            match &timestamps {
                Some(timestamp) if self.uuid != timestamp.uuid => {
                    return Err(ParseError(ObjectKind::Thumbnail, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "ThumbnailTimestamps", uuid_main: self.uuid, uuid_struct: timestamps.unwrap().uuid.clone() })));
                },
                _ => {},
            };
            if self.uuid != votes.uuid {
                return Err(ParseError(ObjectKind::Thumbnail, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "ThumbnailVotes", uuid_main: self.uuid, uuid_struct: votes.uuid.clone() })));
            }
            let mut flags = ThumbnailFlags::empty();
            flags.set(ThumbnailFlags::Original, intbool!(thumb self, original));
            flags.set(ThumbnailFlags::Locked, intbool!(thumb votes, locked));
            flags.set(ThumbnailFlags::ShadowHidden, intbool!(thumb votes, shadow_hidden));
            flags.set(ThumbnailFlags::Removed, intbool!(thumb votes, removed));
            if !flags.contains(ThumbnailFlags::Original) && timestamps.is_none() {
                return Err(ParseError(ObjectKind::Thumbnail, Box::new(ParseErrorKind::MissingSubobject { struct_name: "ThumbnailTimestamps", uuid: self.uuid })));
            }
            Ok(super::Thumbnail{
                uuid: self.uuid,
                video_id: self.video_id,
                user_id: self.user_id,
                time_submitted: self.time_submitted,
                timestamp: timestamps.map(|t| t.timestamp),
                votes: votes.votes,
                downvotes: votes.downvotes,
                flags,
            })
        }
    }

    impl Title {
        pub fn try_merge(self, votes: &TitleVotes) -> Result<super::Title> {
            if self.uuid != votes.uuid {
                return Err(ParseError(ObjectKind::Title, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "TitleVotes", uuid_main: self.uuid, uuid_struct: votes.uuid.clone() })));
            }
            let mut flags = TitleFlags::empty();
            flags.set(TitleFlags::Original, intbool!(title self, original));
            flags.set(TitleFlags::Locked, intbool!(title votes, locked));
            flags.set(TitleFlags::ShadowHidden, intbool!(title votes, shadow_hidden));
            flags.set(TitleFlags::Unverified, intbool!(title votes, verification, 0, -1));
            flags.set(TitleFlags::Removed, intbool!(title votes, removed));
            Ok(super::Title{
                uuid: self.uuid,
                video_id: self.video_id,
                title: self.title,
                user_id: self.user_id,
                time_submitted: self.time_submitted,
                votes: votes.votes,
                downvotes: votes.downvotes,
                flags,
            })
        }
    }

    impl TryFrom<Username> for super::Username {
        type Error = ParseError;

        fn try_from(value: Username) -> Result<super::Username> {
            let locked = intbool!(uname value, locked);
            Ok(super::Username {
                user_id: value.user_id,
                username: value.username,
                locked,
            })
        }
    }

    impl Dedupe for Thumbnail {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.uuid);
            set.dedupe_arc(&mut self.video_id);
            set.dedupe_arc(&mut self.user_id);
        }
    }
    impl Dedupe for Title {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.uuid);
            set.dedupe_arc(&mut self.title);
            set.dedupe_arc(&mut self.video_id);
            set.dedupe_arc(&mut self.user_id);
        }
    }
    impl Dedupe for ThumbnailVotes {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.uuid);
        }
    }
    impl Dedupe for ThumbnailTimestamps {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.uuid);
        }
    }
    impl Dedupe for TitleVotes {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.uuid);
        }
    }
    impl Dedupe for VIPUser {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.user_id);
        }
    }
    impl Dedupe for Username {
        fn dedupe(&mut self, set: &mut StringSet) {
            set.dedupe_arc(&mut self.user_id);
            set.dedupe_arc(&mut self.username);
        }
    }
}
