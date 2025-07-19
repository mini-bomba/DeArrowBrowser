/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
*
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::{
    collections::{hash_map::Entry, HashMap}, fs::File, path::{Path, PathBuf}, sync::Arc
};

use cloneable_errors::{ErrContext, ErrorContext, ResContext};
use log::info;

use crate::{
    csv::{parsing::WithWarnings, types as csv_types}, dedupe::{arc_addr, AddrArc, Dedupe, StringSet}, errors::{ObjectKind, ParseError, ParseErrorKind}, types::*
};

type Result<T> = std::result::Result<T, ErrorContext>;

pub struct DearrowDB {
    pub titles: Vec<Title>,
    pub thumbnails: Vec<Thumbnail>,
    pub usernames: Vec<Username>,
    pub vip_users: Vec<Arc<str>>,
    /// `VideoInfos` are grouped by hashprefix (a u16 value)
    /// Use `.get_video_info()` to get a specific `VideoInfo` object
    pub video_infos: Box<[Box<[VideoInfo]>]>,
    pub warnings: Vec<Warning>,
    pub casual_titles: Vec<CasualTitle>,
    /// number of usernames which were discarded at the deserialization stage
    pub usernames_skipped: u64,
}

pub struct DBPaths {
    pub thumbnails: PathBuf,
    pub thumbnail_timestamps: PathBuf,
    pub thumbnail_votes: PathBuf,
    pub titles: PathBuf,
    pub title_votes: PathBuf,
    pub usernames: PathBuf,
    pub vip_users: PathBuf,
    pub sponsor_times: PathBuf,
    pub warnings: PathBuf,
    pub casual_votes: PathBuf,
    pub casual_vote_titles: PathBuf,
}

pub type LoadResult = (DearrowDB, Vec<ErrorContext>);

impl DearrowDB {
    fn sort(&mut self) {
        self.titles.sort_unstable_by_key(|v| v.time_submitted);
        self.thumbnails.sort_unstable_by_key(|v| v.time_submitted);
        self.usernames.sort_unstable_by_key(|v| arc_addr(&v.user_id));
        self.vip_users.sort_unstable_by_key(arc_addr);
        self.casual_titles.sort_unstable_by_key(|v| v.first_submitted);
    }

    pub fn get_video_info(&self, video_id: &Arc<str>) -> Option<&VideoInfo> {
        self.video_infos[compute_hashprefix(video_id) as usize]
            .iter()
            .find(|v| Arc::ptr_eq(&v.video_id, video_id))
    }

    pub fn get_username(&self, user_id: &Arc<str>) -> Option<&Username> {
        self.usernames
            .binary_search_by_key(&arc_addr(user_id), |u| arc_addr(&u.user_id))
            .ok()
            .map(|i| &self.usernames[i])
    }

    pub fn is_vip(&self, user_id: &Arc<str>) -> bool {
        self.vip_users
            .binary_search_by_key(&arc_addr(user_id), arc_addr)
            .is_ok()
    }

    pub fn load_dir(dir: &Path, string_set: &mut StringSet, load_all_usernames: bool) -> Result<LoadResult> {
        DearrowDB::load(
            &DBPaths {
                thumbnails:           dir.join("thumbnails.csv"),
                thumbnail_timestamps: dir.join("thumbnailTimestamps.csv"),
                thumbnail_votes:      dir.join("thumbnailVotes.csv"),
                titles:               dir.join("titles.csv"),
                title_votes:          dir.join("titleVotes.csv"),
                usernames:            dir.join("userNames.csv"),
                vip_users:            dir.join("vipUsers.csv"),
                sponsor_times:        dir.join("sponsorTimes.csv"),
                warnings:             dir.join("warnings.csv"),
                casual_votes:         dir.join("casualVotes.csv"),
                casual_vote_titles:   dir.join("casualVoteTitles.csv"),
            },
            string_set,
            load_all_usernames,
        )
    }

    pub fn load(paths: &DBPaths, string_set: &mut StringSet, load_all_usernames: bool) -> Result<LoadResult> {
        // Briefly open each file in read-only to check if they exist before continuing to parse
        File::open(&paths.thumbnails).context("Could not open the thumbnails file")?;
        File::open(&paths.thumbnail_timestamps)
            .context("Could not open the thumbnail timestamps file")?;
        File::open(&paths.thumbnail_votes).context("Could not open the thumbnail votes file")?;
        File::open(&paths.titles).context("Could not open the titles file")?;
        File::open(&paths.title_votes).context("Could not open the title votes file")?;
        File::open(&paths.usernames).context("Could not open the usernames file")?;
        File::open(&paths.vip_users).context("Could not open the VIP users file")?;
        File::open(&paths.sponsor_times)
            .context("Could not open the SponsorBlock segments file")?;
        File::open(&paths.warnings).context("Could not open the warnings file")?;

        // Create a vec for non-fatal deserialization errors
        let mut errors: Vec<ErrorContext> = Vec::new();

        info!("Loading thumbnails...");
        let thumbnails = Self::load_thumbnails(paths, string_set, &mut errors)?;

        info!("Loading titles...");
        let titles = Self::load_titles(paths, string_set, &mut errors)?;

        info!("Loading casual titles...");
        let casual_titles = Self::load_casual_titles(paths, string_set, &mut errors)?;

        info!("Loading VIPs...");
        let vip_users = Self::load_vips(paths, string_set, &mut errors)?;

        info!("Extracting video info from SponsorBlock segments...");
        let video_infos = Self::load_video_info(paths, string_set, &mut errors)?;

        info!("Loading warnings...");
        let warnings = Self::load_warnings(paths, string_set, &mut errors)?;

        info!("Loading usernames...");
        let (usernames, usernames_skipped) = Self::load_usernames(paths, string_set, &mut errors, load_all_usernames)?;

        info!("Sorting the database...");
        let mut data = DearrowDB {
            titles,
            thumbnails,
            usernames,
            vip_users,
            video_infos,
            warnings,
            casual_titles,
            usernames_skipped,
        };
        data.sort();

        info!("DearrowDB loaded!");
        Ok((data, errors))
    }

    fn load_thumbnails(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Vec<Thumbnail>> {
        // Load the entirety of thumbnailTimestamps and thumbnailVotes into HashMaps, while
        // deduplicating strings
        let thumbnail_timestamps: HashMap<AddrArc<str>, csv_types::ThumbnailTimestamps> =
            csv::Reader::from_path(&paths.thumbnail_timestamps)
                .context("Could not initialize csv reader for thumbnail timestamps")?
                .into_deserialize::<csv_types::ThumbnailTimestamps>()
                .filter_map(|result| {
                    match result.context("Error while deserializing thumbnail timestamps") {
                        Ok(mut thumb) => {
                            thumb.dedupe(string_set);
                            Some(thumb)
                        }
                        Err(error) => {
                            errors.push(error);
                            None
                        }
                    }
                })
                .map(|timestamp| (timestamp.uuid.clone().into(), timestamp))
                .collect();
        let thumbnail_votes: HashMap<AddrArc<str>, csv_types::ThumbnailVotes> =
            csv::Reader::from_path(&paths.thumbnail_votes)
                .context("Could not initialize csv reader for thumbnail votes")?
                .into_deserialize::<csv_types::ThumbnailVotes>()
                .filter_map(|result| {
                    match result.context("Error while deserializing thumbnail votes") {
                        Ok(mut thumb) => {
                            thumb.dedupe(string_set);
                            Some(thumb)
                        }
                        Err(error) => {
                            errors.push(error);
                            None
                        }
                    }
                })
                .map(|thumb| (thumb.uuid.clone().into(), thumb))
                .collect();

        // Load the Thumbnail objects while deduplicating strings and merging them with other Thumbnail* objects
        Ok(csv::Reader::from_path(&paths.thumbnails)
            .context("Could not initialize csv reader for thumbnails")?
            .into_deserialize::<csv_types::Thumbnail>()
            .filter_map(
                |result| match result.context("Error while deserializing thumbnails") {
                    Ok(mut thumb) => {
                        thumb.dedupe(string_set);
                        let uuid = thumb.uuid.clone().into();
                        let timestamp = thumbnail_timestamps.get(&uuid);
                        let votes = thumbnail_votes.get(&uuid);
                        match thumb.try_merge(timestamp, votes) {
                            Ok(WithWarnings { obj, warnings }) => {
                                errors.extend(
                                    warnings
                                        .into_iter()
                                        .map(|e| e.context("Warning from merging thumbnail data")),
                                );
                                Some(obj)
                            }
                            Err(err) => {
                                errors.push(err.context("Error while merging thumbnail data"));
                                None
                            }
                        }
                    }
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect())
    }

    fn load_titles(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Vec<Title>> {
        let title_votes: HashMap<AddrArc<str>, csv_types::TitleVotes> =
            csv::Reader::from_path(&paths.title_votes)
                .context("Could not initialize csv reader for title votes")?
                .into_deserialize::<csv_types::TitleVotes>()
                .filter_map(|result| {
                    match result.context("Error while deserializing title votes") {
                        Ok(mut title) => {
                            title.dedupe(string_set);
                            Some(title)
                        }
                        Err(error) => {
                            errors.push(error);
                            None
                        }
                    }
                })
                .map(|title| (title.uuid.clone().into(), title))
                .collect();
        Ok(csv::Reader::from_path(&paths.titles)
            .context("Could not initialize csv reader for titles")?
            .into_deserialize::<csv_types::Title>()
            .filter_map(
                |result| match result.context("Error while deserializing titles") {
                    Ok(mut title) => {
                        title.dedupe(string_set);
                        let votes = title_votes.get(&title.uuid.clone().into());
                        match title.try_merge(votes) {
                            Ok(WithWarnings { obj, warnings }) => {
                                errors.extend(
                                    warnings
                                        .into_iter()
                                        .map(|e| e.context("Warning from merging title data")),
                                );
                                Some(obj)
                            }
                            Err(err) => {
                                errors.push(err.context("Error while merging title data"));
                                None
                            }
                        }
                    }
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect())
    }

    fn load_casual_titles(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Vec<CasualTitle>> {
        let mut titles: HashMap<(usize, i8), CasualTitle> = 
            csv::Reader::from_path(&paths.casual_vote_titles)
                .context("Could not initialize csv reader for casual vote titles")?
                .into_deserialize::<csv_types::CasualTitle>()
                .filter_map(|result| 
                    match result.context("Error while deserializing casual vote titles") {
                        Ok(mut title) => {
                            title.dedupe(string_set);
                            Some(((arc_addr(&title.video_id), title.id), title.into()))
                        }
                        Err(error) => {
                            errors.push(error);
                            None
                        }
                    }
                )
                .collect();
        csv::Reader::from_path(&paths.casual_votes)
            .context("Could not initialize csv reader for casual votes")?
            .into_deserialize::<csv_types::CasualVote>()
            .for_each(|result| {
                let mut vote = match result.context("Error while deserializing casual votes") {
                    Ok(vote) => vote,
                    Err(error) => return errors.push(error),
                };
                vote.dedupe(string_set);
                match titles.entry((arc_addr(&vote.video_id), vote.title_id)) {
                    Entry::Vacant(entry) => drop(entry.insert(vote.into())),
                    Entry::Occupied(entry) => entry.into_mut().add_vote(vote).context("Error while merging casual votes").unwrap_or_else(|err| errors.push(err)),
                }
            });
        Ok(
            titles.into_values()
                .filter(|title| {
                    if title.votes.values().all(Option::is_none) {
                        errors.push(ParseError(
                            ObjectKind::CasualTitle,
                            ParseErrorKind::CasualTitleWithoutVotes {
                                video_id: title.video_id.clone(),
                                title: title.title.clone(),
                            }
                        ).context("Error while finalizing casual vote merge"));
                        false
                    } else {
                        true
                    }
                })
                .collect()
        )
    }

    fn load_usernames(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
        load_all: bool,
    ) -> Result<(Vec<Username>, u64)> {
        let mut skip_count: u64 = 0;
        Ok((csv::Reader::from_path(&paths.usernames)
            .context("could not initialize csv reader for usernames")?
            .into_deserialize::<csv_types::Username>()
            .filter_map(
                |result| match result.context("Error while deserializing usernames") {
                    Ok(mut username) => {
                        if !load_all && !string_set.set.contains(&username.user_id) {
                            skip_count = skip_count.saturating_add(1);
                            return None;
                        }
                        username.dedupe(string_set);
                        TryInto::<Username>::try_into(username)
                            .map_err(|e| {
                                errors.push(e.context("Error while parsing username data"));
                            })
                            .ok()
                    }
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect(),
            skip_count
        ))
    }

    fn load_vips(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Vec<Arc<str>>> {
        Ok(csv::Reader::from_path(&paths.vip_users)
            .context("could not initialize csv reader for VIP users")?
            .into_deserialize::<csv_types::VIPUser>()
            .filter_map(
                |result| match result.context("Error while deserializing vip users") {
                    Ok(mut vip) => {
                        vip.dedupe(string_set);
                        Some(vip.user_id)
                    }
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect())
    }

    #[allow(clippy::float_cmp)]
    fn load_video_info(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Box<[Box<[VideoInfo]>]>> {
        const HASHBLOCK_RANGE: std::ops::RangeInclusive<usize> = 0..=u16::MAX as usize;
        let mut segments: Box<[HashMap<AddrArc<str>, Vec<csv_types::TrimmedSponsorTime>>]> =
            HASHBLOCK_RANGE.map(|_| HashMap::new()).collect();
        let mut video_durations: Box<[HashMap<AddrArc<str>, csv_types::VideoDuration>]> =
            HASHBLOCK_RANGE.map(|_| HashMap::new()).collect();
        csv::Reader::from_path(&paths.sponsor_times)
            .context("could not initialize csv reader for SponsorBlock segments")?
            .into_deserialize::<csv_types::SponsorTime>()
            .for_each(|result| {
                match result.context("Error while deserializing SponsorBlock segments") {
                    Ok(segment) => {
                        if let Some((hash_prefix, duration, segment)) = segment.filter_and_split(string_set) {
                            video_durations[hash_prefix as usize]
                                .entry(duration.video_id.clone().into())
                                .and_modify(|d| {
                                    if duration.video_duration != 0.
                                        && (d.time_submitted > duration.time_submitted
                                            || d.video_duration == 0.)
                                    {
                                        let mut duration = duration.clone();
                                        duration.has_outro |= d.has_outro;
                                        *d = duration;
                                    } else {
                                        d.has_outro |= duration.has_outro;
                                    }
                                })
                                .or_insert(duration);
                            segments[hash_prefix as usize]
                                .entry(segment.video_id.clone().into())
                                .or_default()
                                .push(segment);
                        }
                    }
                    Err(error) => errors.push(error),
                }
            });
        Ok(HASHBLOCK_RANGE
            .map(|hash_prefix| {
                video_durations[hash_prefix]
                    .values()
                    .filter_map(|duration| {
                        let vid = duration.video_id.clone().into();
                        let video_duration = if duration.video_duration > 0. {
                            duration.video_duration
                        } else {
                            match segments[hash_prefix]
                                .get(&vid)
                                .and_then(|l| l.iter().map(|s| s.end_time).max_by(f64::total_cmp))
                            {
                                None => return None, // no duration, no segments - no data
                                Some(d) => d,
                            }
                        };
                        Some(VideoInfo {
                            video_id: duration.video_id.clone(),
                            video_duration: duration.video_duration,
                            uncut_segments: match segments[hash_prefix].get_mut(&vid)
                            {
                                None => Box::new([UncutSegment {
                                    offset: 0.,
                                    length: 1.,
                                }]),
                                Some(segments) => {
                                    segments.sort_unstable_by(|a, b| {
                                        a.start_time.total_cmp(&b.start_time)
                                    });
                                    let mut uncut_segments: Vec<UncutSegment> = vec![];
                                    for segment in segments {
                                        if segment.start_time >= video_duration {
                                            continue;
                                        }
                                        let offset = segment.start_time / video_duration;
                                        let end =
                                            segment.end_time.min(video_duration) / video_duration;
                                        if let Some(last_segment) = uncut_segments.last_mut() {
                                            // segment already included in previous one
                                            if last_segment.offset > end {
                                                continue;
                                            }
                                            // segment overlaps previous one, but extends past its
                                            // end time
                                            if last_segment.offset > offset {
                                                *last_segment = UncutSegment {
                                                    offset: end,
                                                    length: 1. - end,
                                                };
                                            // segment does not overlap previous one
                                            } else {
                                                *last_segment = UncutSegment {
                                                    offset: last_segment.offset,
                                                    length: offset - last_segment.offset,
                                                };
                                                uncut_segments.push(UncutSegment {
                                                    offset: end,
                                                    length: 1. - end,
                                                });
                                            }
                                        } else {
                                            if offset != 0. {
                                                uncut_segments.push(UncutSegment {
                                                    offset: 0.,
                                                    length: offset,
                                                });
                                            }
                                            if segment.end_time != video_duration {
                                                uncut_segments.push(UncutSegment {
                                                    offset: end,
                                                    length: 1. - end,
                                                });
                                            }
                                        }
                                    }
                                    if let Some(segment) = uncut_segments.last() {
                                        if segment.offset == 1. {
                                            uncut_segments.pop();
                                        }
                                    } else {
                                        uncut_segments.push(UncutSegment {
                                            offset: 0.,
                                            length: 1.,
                                        });
                                    }
                                    uncut_segments.into_iter().collect()
                                }
                            },
                            has_outro: duration.has_outro,
                        })
                    })
                    .collect()
            })
            .collect())
    }

    fn load_warnings(
        paths: &DBPaths,
        string_set: &mut StringSet,
        errors: &mut Vec<ErrorContext>,
    ) -> Result<Vec<Warning>> {
        const CONTEXT: &str = "Error while deserializing warnings";
        Ok(csv::Reader::from_path(&paths.warnings)
            .context("could not initialize csv reader for warnings")?
            .into_deserialize::<csv_types::Warning>()
            .filter_map(|result| {
                match result
                    .context(CONTEXT)
                    .and_then(|w| Warning::try_from(w).context(CONTEXT))
                {
                    Ok(mut tip) => {
                        tip.dedupe(string_set);
                        Some(tip)
                    }
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                }
            })
            .collect())
    }
}
