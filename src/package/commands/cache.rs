//! `yaoxiang cache` command - Manage the global package cache (RFC-014 Phase 3)

use crate::package::cache::GlobalCache;
use crate::package::error::PackageResult;
use crate::util::i18n::{t, t_simple, current_lang, MSG};

/// `yaoxiang cache clean`：清空全局缓存目录
pub fn clean() -> PackageResult<()> {
    let cache = GlobalCache::from_config()?;
    let lang = current_lang();

    match cache.clean()? {
        Some(bytes) => println!(
            "{}",
            t(
                MSG::PackageCacheCleaned,
                lang,
                Some(&[&GlobalCache::format_size(bytes)])
            )
        ),
        None => println!("{}", t_simple(MSG::PackageCacheEmpty, lang)),
    }

    Ok(())
}
