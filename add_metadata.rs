/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
*
*  Some code was copied and adapted from the built library: https://github.com/lukaslueg/built,
*  which is licensed under the MIT license.
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

use std::{env, fs::File, io::{BufWriter, Write}, path::Path};

use chrono::{FixedOffset, TimeZone};
use cloneable_errors::{ErrContext, ErrorContext, ResContext};
use git2::Repository;

fn main() -> Result<(), ErrorContext> {
    let built_file = Path::new(&env::var("OUT_DIR").context("OUT_DIR not set")?).join("built.rs");
    let manifest_location = env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?;
    let manifest_location = manifest_location.as_ref();

    // Compile standard built info
    built::write_built_file_with_opts(
        Some(manifest_location),
        &built_file,
    ).context("Failed to compile build-time info")?;

    // Open the file to append more
    let mut file = BufWriter::new(File::options().append(true).open(&built_file).context("Failed to open the build-time info file")?);

    // Add custom info
    extra_git_info(&mut file, manifest_location).context("Failed to compile extra build-time git info")?;
    Ok(())
}

fn extra_git_info(file: &mut BufWriter<File>, manifest_loc: &Path) -> Result<(), ErrorContext> {
    let commit_timestamp = match Repository::discover(manifest_loc) {
        Err(e) if e.class() == git2::ErrorClass::Repository && e.code() == git2::ErrorCode::NotFound => "None".to_owned(),
        Err(e) => return Err(e.context("Failed to read git repo")),
        Ok(repo) => {
            let head = repo
                .head().context("Failed to read repo head")?
                .peel_to_commit().context("Failed to peel reference to commit")?;
            let commit_time = head.time();
            let tz = FixedOffset::east_opt(commit_time.offset_minutes() * 60).context("Commit timestamp had an invalid timezone offset")?;
            // fixed offset should never be invalid or ambigious
            let commit_datetime = tz.timestamp_opt(commit_time.seconds(), 0).unwrap(); 

            format!("Some(\"{}\")", commit_datetime.to_rfc3339().escape_default())
        },
    };
    writeln!(file, "\
        #[allow(clippy::needless_raw_string_hashes)]\n\
        #[doc=r#\"The commit time in RFC3339/ISO8601.\"#]\n\
        #[allow(dead_code)]\n\
        pub const GIT_COMMIT_TIMESTAMP: Option<&str> = {commit_timestamp};"
    ).context("Failed to write data to file")?;
    Ok(())
}
