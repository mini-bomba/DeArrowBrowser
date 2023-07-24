use std::{sync::Arc, fmt::Display, collections::{HashSet, HashMap}, path::Path, fs::File};
use bitflags::bitflags;
use anyhow::{Result, Context, Error};

bitflags! {
    #[derive(Clone, Copy)]
    pub struct ThumbnailFlags: u8 {
        const Original     = 0b00000001;
        const Locked       = 0b00000010;
        const ShadowHidden = 0b00000100;
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct TitleFlags: u8 {
        const Original     = 0b00000001;
        const Locked       = 0b00000010;
        const ShadowHidden = 0b00000100;
        const Unverified   = 0b00001000;
    }
}

#[derive(Clone)]
pub struct Thumbnail {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub timestamp: Option<f64>,
    pub votes: i8,
    pub flags: ThumbnailFlags,
}

#[derive(Clone)]
pub struct Title {
    pub uuid: Arc<str>,
    pub video_id: Arc<str>,
    pub title: Arc<str>,
    pub user_id: Arc<str>,
    pub time_submitted: i64,
    pub votes: i8,
    pub flags: TitleFlags,
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
pub enum SubmissionKind {
    Title,
    Thumbnail,
}

#[derive(Debug)]
pub struct ParseError(SubmissionKind, Box<ParseErrorKind>);

impl Display for SubmissionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmissionKind::Title => write!(f, "Title"),
            SubmissionKind::Thumbnail => write!(f, "Thumbnail"),
        }
    }
}

impl std::error::Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let submission_kind = &self.0;
        match *self.1 {
            ParseErrorKind::InvalidValue { ref uuid, ref field, ref value } => write!(f, "Parsing error: Field {field} in {submission_kind} {uuid} contained an invalid value: {value}"),
            ParseErrorKind::MismatchedUUIDs { ref struct_name, ref uuid_main, ref uuid_struct } => write!(f, "Merge error: Component {struct_name} of {submission_kind} {uuid_main} had a different UUID: {uuid_struct}"),
            ParseErrorKind::MissingSubobject { ref struct_name, ref uuid } => write!(f, "Parsing error: {submission_kind} {uuid} was missing an associated {struct_name} object")
        }
    }
}

pub struct DearrowDB {
    pub titles: HashMap<Arc<str>, Title>,
    pub thumbnails: HashMap<Arc<str>, Thumbnail>,
}

pub type LoadResult = (DearrowDB, Vec<Error>);

impl DearrowDB {
    pub fn load_dir(dir: &Path, string_set: &mut StringSet) -> Result<LoadResult> {
        DearrowDB::load(
            &dir.join("thumbnails.csv"),
            &dir.join("thumbnailTimestamps.csv"),
            &dir.join("thumbnailVotes.csv"),
            &dir.join("titles.csv"),
            &dir.join("titleVotes.csv"),
            string_set,
        )
    }

    pub fn load(thumbnails_path: &Path, thumbnail_timestamps_path: &Path, thumbnail_votes_path: &Path, titles_path: &Path, title_votes_path: &Path, string_set: &mut StringSet) -> Result<LoadResult> {
        // Briefly open each file in read-only to check if they exist before continuing to parse
        drop(File::open(thumbnails_path).context("Could not open the thumbnails file")?);
        drop(File::open(thumbnail_timestamps_path).context("Could not open the thumbnail timestamps file")?);
        drop(File::open(thumbnail_votes_path).context("Could not open the thumbnail votes file")?);
        drop(File::open(titles_path).context("Could not open the titles file")?);
        drop(File::open(title_votes_path).context("Could not open the title votes file")?);

        // Create a vec for non-fatal deserialization errors
        let mut errors: Vec<Error> = Vec::new();
        
        // Load the entirety of thumbnailTimestamps and thumbnailVotes into HashMaps, while
        // deduplicating strings
        let thumbnail_timestamps: HashMap<Arc<str>, csv_data::ThumbnailTimestamps> = csv::Reader::from_path(thumbnail_timestamps_path)
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
        let thumbnail_votes: HashMap<Arc<str>, csv_data::ThumbnailVotes> = csv::Reader::from_path(thumbnail_votes_path)
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
        let thumbnails: HashMap<Arc<str>, Thumbnail> = csv::Reader::from_path(thumbnails_path)
            .context("Could not initialize csv reader for thumbnails")?
            .into_deserialize::<csv_data::Thumbnail>()
            .filter_map(|result| match result.context("Error while deserializing thumbnails") {
                Ok(mut thumb) => {
                    thumb.dedupe(string_set);
                    let timestamp = thumbnail_timestamps.get(&thumb.uuid);
                    let votes = match thumbnail_votes.get(&thumb.uuid) {
                        Some(v) => v,
                        None => {
                            errors.push(Error::new(ParseError(SubmissionKind::Thumbnail, Box::new(ParseErrorKind::MissingSubobject { struct_name: "ThumbnailVotes", uuid: thumb.uuid.clone() }))));
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
            .map(|thumb| (thumb.uuid.clone(), thumb))
            .collect();

        drop(thumbnail_timestamps);
        drop(thumbnail_votes);

        // Do the same for titles
        let title_votes: HashMap<Arc<str>, csv_data::TitleVotes> = csv::Reader::from_path(title_votes_path)
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
        let titles: HashMap<Arc<str>, Title> = csv::Reader::from_path(titles_path)
            .context("Could not initialize csv reader for titles")?
            .into_deserialize::<csv_data::Title>()
            .filter_map(|result| match result.context("Error while deserializing titles") {
                Ok(mut title) => {
                    title.dedupe(string_set);
                    let votes = match title_votes.get(&title.uuid) {
                        Some(v) => v,
                        None => {
                            errors.push(Error::new(ParseError(SubmissionKind::Title, Box::new(ParseErrorKind::MissingSubobject { struct_name: "TitleVotes", uuid: title.uuid.clone() }))));
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
            .map(|title| (title.uuid.clone(), title))
            .collect();

        drop(title_votes);

        Ok((DearrowDB {titles, thumbnails}, errors))
    }
}


mod csv_data {
    use std::sync::Arc;
    use serde::Deserialize;
    use super::{ParseError, SubmissionKind, ParseErrorKind, ThumbnailFlags, TitleFlags, StringSet, Dedupe};

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
    }

    macro_rules! intbool {
        (thumb $struct:expr, $field:ident) => {
            intbool!(! $struct, $field, SubmissionKind::Thumbnail, 0, 1)
        };
        (title $struct:expr, $field:ident) => {
            intbool!(! $struct, $field, SubmissionKind::Title, 0, 1)
        };
        (thumb $struct:expr, $field:ident, $falseint: expr, $trueint:expr) => {
            intbool!(! $struct, $field, SubmissionKind::Thumbnail, $falseint, $trueint)
        };
        (title $struct:expr, $field:ident, $falseint: expr, $trueint:expr) => {
            intbool!(! $struct, $field, SubmissionKind::Title, $falseint, $trueint)
        };
        (! $struct:expr, $field:ident, $kind:expr, $falseint:expr, $trueint:expr) => {
            match $struct.$field {
                $falseint => false,
                $trueint => true,
                _ => return Err(ParseError($kind, Box::new(ParseErrorKind::InvalidValue { uuid: $struct.uuid.clone(), field: stringify!($field), value: $struct.$field }))),
            }
        };
    }


    impl Thumbnail {
        pub fn try_merge(self, timestamps: Option<&ThumbnailTimestamps>, votes: &ThumbnailVotes) -> Result<super::Thumbnail> {
            match &timestamps {
                Some(timestamp) if self.uuid != timestamp.uuid => {
                    return Err(ParseError(SubmissionKind::Thumbnail, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "ThumbnailTimestamps", uuid_main: self.uuid, uuid_struct: timestamps.unwrap().uuid.clone() })));
                },
                _ => {},
            };
            if self.uuid != votes.uuid {
                return Err(ParseError(SubmissionKind::Thumbnail, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "ThumbnailVotes", uuid_main: self.uuid, uuid_struct: votes.uuid.clone() })));
            }
            let mut flags = ThumbnailFlags::empty();
            flags.set(ThumbnailFlags::Original, intbool!(thumb self, original));
            flags.set(ThumbnailFlags::Locked, intbool!(thumb votes, locked));
            flags.set(ThumbnailFlags::ShadowHidden, intbool!(thumb votes, shadow_hidden));
            if !flags.contains(ThumbnailFlags::Original) && timestamps.is_none() {
                return Err(ParseError(SubmissionKind::Thumbnail, Box::new(ParseErrorKind::MissingSubobject { struct_name: "ThumbnailTimestamps", uuid: self.uuid })));
            }
            Ok(super::Thumbnail{
                uuid: self.uuid,
                video_id: self.video_id,
                user_id: self.user_id,
                time_submitted: self.time_submitted,
                timestamp: timestamps.map(|t| t.timestamp),
                votes: votes.votes,
                flags,
            })
        }
    }

    impl Title {
        pub fn try_merge(self, votes: &TitleVotes) -> Result<super::Title> {
            if self.uuid != votes.uuid {
                return Err(ParseError(SubmissionKind::Title, Box::new(ParseErrorKind::MismatchedUUIDs { struct_name: "TitleVotes", uuid_main: self.uuid, uuid_struct: votes.uuid.clone() })));
            }
            let mut flags = TitleFlags::empty();
            flags.set(TitleFlags::Original, intbool!(title self, original));
            flags.set(TitleFlags::Locked, intbool!(title votes, locked));
            flags.set(TitleFlags::ShadowHidden, intbool!(title votes, shadow_hidden));
            flags.set(TitleFlags::Unverified, intbool!(title votes, verification, 0, -1));
            Ok(super::Title{
                uuid: self.uuid,
                video_id: self.video_id,
                title: self.title,
                user_id: self.user_id,
                time_submitted: self.time_submitted,
                votes: votes.votes,
                flags,
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
}
