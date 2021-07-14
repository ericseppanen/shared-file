use std::convert::TryInto;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Deref;
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
use std::sync::Arc;

fn u64_from(x: usize) -> u64 {
    x.try_into().expect("usize should fit in u64")
}

/// A cheaply clone-able File wrapper.
///
/// All clones of `SharedFile` share the same underlying `File`.
/// Each instance of `SharedFile` can perform `Read` and `Seek` operations
/// independently, and maintains its own seek position.
///
pub struct SharedFile<F> {
    file: F,
    pos: u64,
}

/// A `SharedFile` that uses an `Arc<File>` for file access.
///
/// Choose this type if you want automatic management of the lifetime
/// of the underlying `File`, or if the lifetime paramater of
/// [`SharedRefFile`] is troublesome.
///
pub type SharedArcFile = SharedFile<Arc<File>>;

/// A `SharedFile` that uses a `&File` for file access.
///
/// Choose this type if you want the cheapest, fastest code. It will
/// mean convincing the compiler that the underlying `File` will outlive
/// all the `SharedRefFile` instances.
///
/// If that seems tricky, use [`SharedArcFile`] instead.
///
pub type SharedRefFile<'a> = SharedFile<&'a File>;

impl<F> SharedFile<F>
where
    F: Clone + Deref<Target = File>,
{
    pub fn new(file: F) -> Self {
        Self {
            file,
            // We don't inherit the previous file position.
            // We could, but it would be more confusing than
            // helpful.
            pos: 0,
        }
    }
}

impl SharedArcFile {
    pub fn new_owned(file: File) -> Self {
        Self {
            file: Arc::new(file),
            // We don't inherit the previous file position.
            // We could, but it would be more confusing than
            // helpful.
            pos: 0,
        }
    }
}

impl<F> Clone for SharedFile<F>
where
    F: Clone + Deref<Target = File>,
{
    fn clone(&self) -> Self {
        Self {
            file: Clone::clone(&self.file),
            // To be consistent with `new`, don't copy the file position.
            pos: 0,
        }
    }
}

impl<F> Read for SharedFile<F>
where
    F: Deref<Target = File>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.file.read_at(buf, self.pos)?;
        self.pos += u64_from(bytes_read);
        Ok(bytes_read)
    }
}

impl<F> Seek for SharedFile<F>
where
    F: Clone + Deref<Target = File>,
{
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        // Add i64 offset to a u64 position.
        fn calc_pos(pos: u64, offset: i64) -> io::Result<u64> {
            // Convert to i64; add the seek offset; convert back to u64.
            // Any failure along the way will be carried along as None,
            // and converted to io::Error at the end.
            let pos: Option<u64> = pos
                .try_into()
                .ok()
                .and_then(|p: i64| p.checked_add(offset))
                .and_then(|p| p.try_into().ok());
            pos.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))
        }

        match pos {
            SeekFrom::Start(spos) => {
                // According to the docs for Seek::seek,
                // "A seek beyond the end of a stream is allowed, but
                // behavior is defined by the implementation."
                self.pos = spos;
                Ok(spos)
            }
            SeekFrom::End(epos) => {
                let file_len = self.file.metadata()?.len();
                calc_pos(file_len, epos)
            }
            SeekFrom::Current(cpos) => calc_pos(self.pos, cpos),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;

    #[test]
    fn ref_read() {
        let buf = "hello world".as_bytes();
        let mut file = tempfile().unwrap();
        file.write_all(buf).unwrap();

        let mut f1 = SharedFile::new(&file);
        let mut f2 = f1.clone();

        let mut s1 = String::new();
        let mut s2 = String::new();
        f1.read_to_string(&mut s1).unwrap();
        f2.read_to_string(&mut s2).unwrap();
    }

    #[test]
    fn arc_read() {
        let buf = "hello world".as_bytes();
        let mut file = tempfile().unwrap();
        file.write_all(buf).unwrap();

        let mut f1 = SharedArcFile::new_owned(file);
        let mut f2 = f1.clone();

        let mut s1 = String::new();
        let mut s2 = String::new();
        f1.read_to_string(&mut s1).unwrap();
        f2.read_to_string(&mut s2).unwrap();
    }
}
