use std::path::Path;
use anyhow::Result;

use dearrow_browser::{DearrowDB, StringSet};

fn main() -> Result<()> {
    let mut string_set = StringSet::with_capacity(8192);
    let (db, errors) = DearrowDB::load_dir(Path::new("/tmp/"), &mut string_set)?;
    for error in errors.iter() {
        println!("{error:?}");
    }
    println!("Loaded {} titles, {} thumbnails. Encountered {} errors", db.titles.len(), db.thumbnails.len(), errors.len());
    Ok(())
}
