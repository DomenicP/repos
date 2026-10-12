use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

pub fn discover(root: &Path, max_depth: Option<usize>) -> (Vec<PathBuf>, Vec<String>) {
    let mut paths = Vec::new();
    let mut errors = Vec::new();
    let mut pending = vec![(root.to_owned(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(format!("discover {directory:?}: {e}"));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    errors.push(format!("discover {directory:?}: {e}"));
                    continue;
                }
            };
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(e) => {
                    errors.push(format!("discover {:?}: {e}", entry.path()));
                    continue;
                }
            };
            if entry.file_name() == ".git" {
                if kind.is_dir() || kind.is_file() {
                    paths.push(directory.clone());
                }
            } else if kind.is_dir() && max_depth.is_none_or(|limit| depth < limit) {
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    paths.sort();
    errors.sort();
    (paths, errors)
}

// Apply an operation once per path, using available CPUs. Results follow the
// input order even when workers finish in a different order.
pub fn run<T: Send>(paths: &[PathBuf], operation: impl Fn(&Path) -> T + Sync) -> Vec<T> {
    let next = AtomicUsize::new(0);
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(paths.len());
    let mut results = thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut results = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(index) else {
                            break;
                        };
                        results.push((index, operation(path)));
                    }
                    results
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("workspace worker panicked"))
            .collect::<Vec<_>>()
    });
    results.sort_unstable_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, result)| result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_preserves_order_and_visits_each_path_once() {
        let paths: Vec<_> = (0..32).map(|i| PathBuf::from(i.to_string())).collect();
        let visits: Vec<_> = paths.iter().map(|_| AtomicUsize::new(0)).collect();
        let results = run(&paths, |path| {
            let index: usize = path
                .to_str()
                .expect("test path is UTF-8")
                .parse()
                .expect("test path is an index");
            visits[index].fetch_add(1, Ordering::Relaxed);
            path.to_owned()
        });
        assert_eq!(results, paths);
        assert!(
            visits
                .iter()
                .all(|count| count.load(Ordering::Relaxed) == 1)
        );
        assert!(run::<()>(&[], |_| panic!("empty input must not run an operation")).is_empty());
    }
}
