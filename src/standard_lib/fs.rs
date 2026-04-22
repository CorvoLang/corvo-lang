use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{chown as unix_chown, lchown, FileTypeExt, MetadataExt, PermissionsExt};

#[cfg(target_os = "linux")]
use libc;

pub fn read(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read requires a path"))?;

    fs::read_to_string(path)
        .map(Value::String)
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn read_lines(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read_lines requires a path"))?;

    let content = fs::read_to_string(path).map_err(|e| CorvoError::file_system(e.to_string()))?;
    let lines: Vec<Value> = content
        .lines()
        .map(|l| Value::String(l.to_string()))
        .collect();
    Ok(Value::List(lines))
}

pub fn write(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.write requires a path"))?;

    let content = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.write requires content"))?;

    fs::write(path, content)
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn append(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.append requires a path"))?;

    let content = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.append requires content"))?;

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()))
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn delete(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.delete requires a path"))?;

    if Path::new(path).is_dir() {
        fs::remove_dir_all(path)
            .map(|_| Value::Boolean(true))
            .map_err(|e| CorvoError::file_system(e.to_string()))
    } else {
        fs::remove_file(path)
            .map(|_| Value::Boolean(true))
            .map_err(|e| CorvoError::file_system(e.to_string()))
    }
}

pub fn exists(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.exists requires a path"))?;

    Ok(Value::Boolean(Path::new(path).exists()))
}

pub fn mkdir(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.mkdir requires a path"))?;

    let recursive = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

    if recursive {
        fs::create_dir_all(path)
            .map(|_| Value::Boolean(true))
            .map_err(|e| CorvoError::file_system(e.to_string()))
    } else {
        fs::create_dir(path)
            .map(|_| Value::Boolean(true))
            .map_err(|e| CorvoError::file_system(e.to_string()))
    }
}

pub fn list_dir(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.list_dir requires a path"))?;

    let entries = fs::read_dir(path)
        .map_err(|e| CorvoError::file_system(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| Value::String(entry.file_name().to_string_lossy().to_string()))
        .collect();

    Ok(Value::List(entries))
}

pub fn copy(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let src = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.copy requires a source path"))?;

    let dest = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.copy requires a destination path"))?;

    fs::copy(src, dest)
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn move_file(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let src = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.move requires a source path"))?;

    let dest = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.move requires a destination path"))?;

    fs::rename(src, dest)
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn link(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let src = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.link requires a source path"))?;

    let dest = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.link requires a destination path"))?;

    fs::hard_link(src, dest)
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

#[allow(unused_variables)]
pub fn symlink(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let src = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.symlink requires a source path"))?;

    let dest = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.symlink requires a destination path"))?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)
            .map(|_| Value::Boolean(true))
            .map_err(|e| CorvoError::file_system(e.to_string()))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("fs.symlink is only supported on Unix"))
    }
}

pub fn realpath(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.realpath requires a path"))?;

    fs::canonicalize(path)
        .map(|p| Value::String(p.to_string_lossy().to_string()))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn truncate(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.truncate requires a path"))?;

    let size = args
        .get(1)
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("fs.truncate requires a size"))?
        as u64;

    let f = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| CorvoError::file_system(e.to_string()))?;

    f.set_len(size)
        .map(|_| Value::Boolean(true))
        .map_err(|e| CorvoError::file_system(e.to_string()))
}

pub fn stat(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.stat requires a path"))?;

    let metadata = fs::metadata(path).map_err(|e| CorvoError::file_system(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert("size".to_string(), Value::Number(metadata.len() as f64));
    result.insert("is_dir".to_string(), Value::Boolean(metadata.is_dir()));
    result.insert(
        "permissions".to_string(),
        Value::String(format!("{:?}", metadata.permissions())),
    );
    result.insert(
        "modified_at".to_string(),
        Value::Number(
            metadata
                .modified()
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as f64
                })
                .unwrap_or(0.0),
        ),
    );

    Ok(Value::Map(result))
}

/// Metadata for a single path (same shape as elements of [`read_dir_meta`]).
pub fn read_meta(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path_s = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read_meta requires a path"))?;

    let follow_symlinks = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

    let path = Path::new(path_s.as_str());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_s.clone());

    let is_symlink_entry = fs::symlink_metadata(path)
        .map(|sm| sm.file_type().is_symlink())
        .unwrap_or(false);

    let meta = if follow_symlinks {
        fs::metadata(path)
            .or_else(|_| fs::symlink_metadata(path))
            .map_err(|e| CorvoError::file_system(e.to_string()))?
    } else {
        fs::symlink_metadata(path).map_err(|e| CorvoError::file_system(e.to_string()))?
    };
    let child_s = path_s.clone();

    let mut m: HashMap<String, Value> = HashMap::new();
    m.insert("name".to_string(), Value::String(name));
    m.insert("path".to_string(), Value::String(child_s.clone()));

    let ft = meta.file_type();
    let is_symlink = is_symlink_entry;
    let is_dir = ft.is_dir();
    let is_file = ft.is_file();

    let symlink_target = if is_symlink {
        fs::read_link(path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    m.insert("is_symlink".to_string(), Value::Boolean(is_symlink));
    m.insert("is_dir".to_string(), Value::Boolean(is_dir));
    m.insert("is_file".to_string(), Value::Boolean(is_file));
    m.insert("symlink_target".to_string(), Value::String(symlink_target));

    #[cfg(unix)]
    {
        let mode = meta.mode() & 0o7777;
        m.insert("mode".to_string(), Value::Number(mode as f64));
        m.insert(
            "mode_string".to_string(),
            Value::String(unix_mode_string(&meta, &ft)),
        );
        m.insert("inode".to_string(), Value::Number(meta.ino() as f64));
        m.insert("nlink".to_string(), Value::Number(meta.nlink() as f64));
        m.insert("uid".to_string(), Value::Number(meta.uid() as f64));
        m.insert("gid".to_string(), Value::Number(meta.gid() as f64));
        m.insert("blocks".to_string(), Value::Number(meta.blocks() as f64));
        m.insert(
            "user".to_string(),
            Value::String(
                uzers::get_user_by_uid(meta.uid())
                    .map(|u| u.name().to_string_lossy().to_string())
                    .unwrap_or_else(|| meta.uid().to_string()),
            ),
        );
        m.insert(
            "group".to_string(),
            Value::String(
                uzers::get_group_by_gid(meta.gid())
                    .map(|g| g.name().to_string_lossy().to_string())
                    .unwrap_or_else(|| meta.gid().to_string()),
            ),
        );
        let rdev = meta.rdev();
        m.insert("major".to_string(), Value::Number(unix_major(rdev) as f64));
        m.insert("minor".to_string(), Value::Number(unix_minor(rdev) as f64));
        m.insert(
            "file_type_char".to_string(),
            Value::String(unix_file_type_char(&ft).to_string()),
        );
    }

    #[cfg(not(unix))]
    {
        m.insert("mode".to_string(), Value::Number(0.0));
        m.insert(
            "mode_string".to_string(),
            Value::String(
                if is_dir {
                    "d?????????"
                } else if is_symlink {
                    "l?????????"
                } else {
                    "-?????????"
                }
                .to_string(),
            ),
        );
        m.insert("inode".to_string(), Value::Number(0.0));
        m.insert("nlink".to_string(), Value::Number(1.0));
        m.insert("uid".to_string(), Value::Number(0.0));
        m.insert("gid".to_string(), Value::Number(0.0));
        m.insert("blocks".to_string(), Value::Number(0.0));
        m.insert("user".to_string(), Value::String(String::new()));
        m.insert("group".to_string(), Value::String(String::new()));
        m.insert("major".to_string(), Value::Number(0.0));
        m.insert("minor".to_string(), Value::Number(0.0));
        m.insert(
            "file_type_char".to_string(),
            Value::String(
                if is_dir {
                    "d"
                } else if is_symlink {
                    "l"
                } else {
                    "-"
                }
                .to_string(),
            ),
        );
    }

    m.insert("size".to_string(), Value::Number(meta.len() as f64));
    #[cfg(unix)]
    {
        let mode = meta.mode();
        m.insert(
            "is_executable".to_string(),
            Value::Boolean(mode & 0o111 != 0),
        );
    }
    #[cfg(not(unix))]
    {
        m.insert("is_executable".to_string(), Value::Boolean(false));
    }
    push_times(&mut m, &meta);

    Ok(Value::Map(m))
}

#[allow(unused_variables)]
pub fn mkfifo(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.mkfifo requires a path"))?;

    let mode = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n as u32)
        .unwrap_or(0o644);

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path =
            CString::new(path.as_str()).map_err(|e| CorvoError::invalid_argument(e.to_string()))?;
        unsafe {
            if libc::mkfifo(c_path.as_ptr(), mode as libc::mode_t) != 0 {
                return Err(CorvoError::io(format!(
                    "mkfifo failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        Ok(Value::Boolean(true))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("fs.mkfifo is only supported on Unix"))
    }
}

#[allow(unused_variables)]
pub fn mknod(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.mknod requires a path"))?;

    let mode = args
        .get(1)
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("fs.mknod requires a mode"))?
        as u32;

    let dev = args.get(2).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path =
            CString::new(path.as_str()).map_err(|e| CorvoError::invalid_argument(e.to_string()))?;
        unsafe {
            if libc::mknod(c_path.as_ptr(), mode as libc::mode_t, dev as libc::dev_t) != 0 {
                return Err(CorvoError::io(format!(
                    "mknod failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        Ok(Value::Boolean(true))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("fs.mknod is only supported on Unix"))
    }
}
pub fn read_link(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read_link requires a path"))?;

    let target =
        fs::read_link(path.as_str()).map_err(|e| CorvoError::file_system(e.to_string()))?;
    Ok(Value::String(target.to_string_lossy().to_string()))
}

pub fn mktemp(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use rand::{distributions::Alphanumeric, Rng};
    let template = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .unwrap_or("tmp.XXXXXX");
    let is_dir = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
    let tmp_dir = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let suffix = args
        .get(3)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    let mut rng = rand::thread_rng();

    // Replace XXXXXX in template
    let parts: Vec<&str> = template.split("XXXXXX").collect();
    if parts.len() < 2 {
        return Err(CorvoError::runtime(
            "fs.mktemp: template must contain 'XXXXXX'",
        ));
    }

    // We only replace the FIRST occurrence in GNU mktemp?
    // Actually GNU mktemp replaces the sequence of X's at the end.
    // If multiple blocks of X's exist, it's usually the last set.

    let mut name = template.to_string();
    if let Some(pos) = name.rfind("XXXXXX") {
        let rand_s: String = (0..6).map(|_| rng.sample(Alphanumeric) as char).collect();
        name.replace_range(pos..pos + 6, &rand_s);
    }
    name.push_str(&suffix);

    let final_path = tmp_dir.join(name);
    let path_s = final_path.to_string_lossy().to_string();

    if is_dir {
        fs::create_dir_all(&final_path).map_err(|e| CorvoError::file_system(e.to_string()))?;
    } else {
        fs::File::create(&final_path).map_err(|e| CorvoError::file_system(e.to_string()))?;
    }

    Ok(Value::String(path_s))
}

pub fn read_hex(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use std::io::{Read, Seek, SeekFrom};
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read_hex: path missing"))?;
    let offset = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;
    let size = args.get(2).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;

    let mut f = fs::File::open(path).map_err(|e| CorvoError::file_system(e.to_string()))?;
    f.seek(SeekFrom::Start(offset))
        .map_err(|e| CorvoError::file_system(e.to_string()))?;

    let mut buf = vec![0u8; size];
    let n = f
        .read(&mut buf)
        .map_err(|e| CorvoError::file_system(e.to_string()))?;
    buf.truncate(n);

    let hex_s: String = buf.iter().map(|b| format!("{:02x}", b)).collect();
    Ok(Value::String(hex_s))
}

pub fn write_hex(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use std::io::{Seek, SeekFrom, Write};
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.write_hex: path missing"))?;
    let offset = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;
    let hex_data = args
        .get(2)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.write_hex: data missing"))?;

    let mut bytes = Vec::new();
    for i in (0..hex_data.len()).step_by(2) {
        if i + 2 <= hex_data.len() {
            if let Ok(b) = u8::from_str_radix(&hex_data[i..i + 2], 16) {
                bytes.push(b);
            }
        }
    }

    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| CorvoError::file_system(e.to_string()))?;

    f.seek(SeekFrom::Start(offset))
        .map_err(|e| CorvoError::file_system(e.to_string()))?;
    f.write_all(&bytes)
        .map_err(|e| CorvoError::file_system(e.to_string()))?;

    Ok(Value::Boolean(true))
}

/// Directory entries with metadata suitable for GNU `ls` (uses `lstat` per entry).
pub fn read_dir_meta(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.read_dir_meta requires a path"))?;

    let follow_symlinks = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

    let base = Path::new(path.as_str());
    let rd = fs::read_dir(base).map_err(|e| CorvoError::file_system(e.to_string()))?;

    let mut entries: Vec<Value> = Vec::new();
    for item in rd {
        let item = item.map_err(|e| CorvoError::file_system(e.to_string()))?;
        let name = item.file_name().to_string_lossy().to_string();
        let child_path: PathBuf = base.join(&name);
        let child_s = child_path.to_string_lossy().to_string();

        let entry_is_symlink = fs::symlink_metadata(&child_path)
            .map(|sm| sm.file_type().is_symlink())
            .unwrap_or(false);

        let meta = if follow_symlinks {
            fs::metadata(&child_path)
                .or_else(|_| fs::symlink_metadata(&child_path))
                .map_err(|e| CorvoError::file_system(e.to_string()))?
        } else {
            fs::symlink_metadata(&child_path).map_err(|e| CorvoError::file_system(e.to_string()))?
        };

        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("name".to_string(), Value::String(name.clone()));
        m.insert("path".to_string(), Value::String(child_s.clone()));

        let ft = meta.file_type();
        let is_symlink = entry_is_symlink;
        let is_dir = ft.is_dir();
        let is_file = ft.is_file();

        let symlink_target = if entry_is_symlink {
            fs::read_link(&child_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        m.insert("is_symlink".to_string(), Value::Boolean(is_symlink));
        m.insert("is_dir".to_string(), Value::Boolean(is_dir));
        m.insert("is_file".to_string(), Value::Boolean(is_file));
        m.insert("symlink_target".to_string(), Value::String(symlink_target));

        #[cfg(unix)]
        {
            let mode = meta.mode() & 0o7777;
            m.insert("mode".to_string(), Value::Number(mode as f64));
            m.insert(
                "mode_string".to_string(),
                Value::String(unix_mode_string(&meta, &ft)),
            );
            m.insert("inode".to_string(), Value::Number(meta.ino() as f64));
            m.insert("nlink".to_string(), Value::Number(meta.nlink() as f64));
            m.insert("uid".to_string(), Value::Number(meta.uid() as f64));
            m.insert("gid".to_string(), Value::Number(meta.gid() as f64));
            m.insert("blocks".to_string(), Value::Number(meta.blocks() as f64));
            m.insert(
                "user".to_string(),
                Value::String(
                    uzers::get_user_by_uid(meta.uid())
                        .map(|u| u.name().to_string_lossy().to_string())
                        .unwrap_or_else(|| meta.uid().to_string()),
                ),
            );
            m.insert(
                "group".to_string(),
                Value::String(
                    uzers::get_group_by_gid(meta.gid())
                        .map(|g| g.name().to_string_lossy().to_string())
                        .unwrap_or_else(|| meta.gid().to_string()),
                ),
            );

            let rdev = meta.rdev();
            m.insert("major".to_string(), Value::Number(unix_major(rdev) as f64));
            m.insert("minor".to_string(), Value::Number(unix_minor(rdev) as f64));

            m.insert(
                "file_type_char".to_string(),
                Value::String(unix_file_type_char(&ft).to_string()),
            );
        }

        #[cfg(not(unix))]
        {
            m.insert("mode".to_string(), Value::Number(0.0));
            m.insert(
                "mode_string".to_string(),
                Value::String(
                    if is_dir {
                        "d?????????"
                    } else if is_symlink {
                        "l?????????"
                    } else {
                        "-?????????"
                    }
                    .to_string(),
                ),
            );
            m.insert("inode".to_string(), Value::Number(0.0));
            m.insert("nlink".to_string(), Value::Number(1.0));
            m.insert("uid".to_string(), Value::Number(0.0));
            m.insert("gid".to_string(), Value::Number(0.0));
            m.insert("blocks".to_string(), Value::Number(0.0));
            m.insert("user".to_string(), Value::String(String::new()));
            m.insert("group".to_string(), Value::String(String::new()));
            m.insert("major".to_string(), Value::Number(0.0));
            m.insert("minor".to_string(), Value::Number(0.0));
            m.insert(
                "file_type_char".to_string(),
                Value::String(
                    if is_dir {
                        "d"
                    } else if is_symlink {
                        "l"
                    } else {
                        "-"
                    }
                    .to_string(),
                ),
            );
        }

        m.insert("size".to_string(), Value::Number(meta.len() as f64));
        #[cfg(unix)]
        {
            let mode = meta.mode();
            let ix = mode & 0o111 != 0;
            m.insert("is_executable".to_string(), Value::Boolean(ix));
        }
        #[cfg(not(unix))]
        {
            m.insert("is_executable".to_string(), Value::Boolean(false));
        }

        push_times(&mut m, &meta);

        entries.push(Value::Map(m));
    }

    Ok(Value::List(entries))
}

/// Parent directory path (empty string if none, e.g. root on Unix).
/// For `\".\"` / `\"./\"`, returns the parent of the current working directory so
/// `ls -a` can synthesize a `..` entry (Rust `Path::parent` is `None` for `.`).
pub fn path_parent(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path_s = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.path_parent requires a path"))?;
    let path_norm = path_s.trim_end_matches('/');
    if path_norm.is_empty() || path_norm == "." {
        let out = std::env::current_dir()
            .ok()
            .and_then(|c| c.parent().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_default();
        return Ok(Value::String(out));
    }
    let p = Path::new(path_s.as_str());
    let out = p
        .parent()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(Value::String(out))
}

pub fn path_filename(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.path_filename requires a path"))?;

    Ok(Value::String(
        std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
    ))
}

pub fn path_join(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let mut path = std::path::PathBuf::new();
    for arg in args {
        if let Some(s) = arg.as_string() {
            path.push(s);
        }
    }
    Ok(Value::String(path.to_string_lossy().to_string()))
}

/// Path of `path` relative to `base` (both strings). If `path` is not under `base`, returns `path` unchanged.
pub fn path_relative(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let base_s = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.path_relative requires base path"))?;
    let path_s = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.path_relative requires path"))?;

    let base = Path::new(base_s.as_str());
    let path = Path::new(path_s.as_str());
    let rel = match path.strip_prefix(base) {
        Ok(r) => r.to_string_lossy().to_string().replace('\\', "/"),
        Err(_) => path_s.to_string(),
    };
    if rel.is_empty() {
        Ok(Value::String(".".to_string()))
    } else {
        Ok(Value::String(rel))
    }
}

#[cfg(unix)]
fn unix_major(dev: u64) -> u32 {
    ((((dev & 0xfff00) >> 8) | ((dev & 0xfffff00000000000) >> 32)) & 0xffffffff) as u32
}

#[cfg(unix)]
fn unix_minor(dev: u64) -> u32 {
    (((dev & 0xff) | ((dev >> 12) & 0xffffff00)) & 0xffffffff) as u32
}

#[cfg(unix)]
fn push_times(m: &mut HashMap<String, Value>, meta: &fs::Metadata) {
    m.insert("mtime_sec".to_string(), Value::Number(meta.mtime() as f64));
    m.insert(
        "mtime_nsec".to_string(),
        Value::Number(meta.mtime_nsec() as f64),
    );
    m.insert("atime_sec".to_string(), Value::Number(meta.atime() as f64));
    m.insert(
        "atime_nsec".to_string(),
        Value::Number(meta.atime_nsec() as f64),
    );
    m.insert("ctime_sec".to_string(), Value::Number(meta.ctime() as f64));
    m.insert(
        "ctime_nsec".to_string(),
        Value::Number(meta.ctime_nsec() as f64),
    );
}

#[cfg(not(unix))]
fn push_times(m: &mut HashMap<String, Value>, meta: &fs::Metadata) {
    // Windows uses `io::Error` for these; other non-Unix targets may differ—`Option` erases the
    // error type.
    fn split_system_time(t: Option<std::time::SystemTime>) -> (f64, f64) {
        let Some(st) = t else {
            return (0.0, 0.0);
        };
        match st.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => (d.as_secs() as f64, d.subsec_nanos() as f64),
            Err(_) => (0.0, 0.0),
        }
    }

    let (mts, mtn) = split_system_time(meta.modified().ok());
    let (ats, atn) = split_system_time(meta.accessed().ok());
    let (cts, ctn) = split_system_time(meta.created().ok());

    m.insert("mtime_sec".to_string(), Value::Number(mts));
    m.insert("mtime_nsec".to_string(), Value::Number(mtn));
    m.insert("atime_sec".to_string(), Value::Number(ats));
    m.insert("atime_nsec".to_string(), Value::Number(atn));
    m.insert("ctime_sec".to_string(), Value::Number(cts));
    m.insert("ctime_nsec".to_string(), Value::Number(ctn));
}

#[cfg(unix)]
fn unix_file_type_char(ft: &fs::FileType) -> char {
    if ft.is_symlink() {
        'l'
    } else if ft.is_dir() {
        'd'
    } else if ft.is_file() {
        '-'
    } else if ft.is_fifo() {
        'p'
    } else if ft.is_socket() {
        's'
    } else if ft.is_block_device() {
        'b'
    } else if ft.is_char_device() {
        'c'
    } else {
        '?'
    }
}

#[cfg(unix)]
fn unix_mode_string(meta: &fs::Metadata, ft: &fs::FileType) -> String {
    let mode = meta.mode();
    let mut s = String::with_capacity(10);
    s.push(unix_file_type_char(ft));
    let r = |m: u32, bit: u32| if m & bit != 0 { 'r' } else { '-' };
    let w = |m: u32, bit: u32| if m & bit != 0 { 'w' } else { '-' };
    let xb = |m: u32, bit: u32| m & bit != 0;

    let ur = r(mode, 0o400);
    let uw = w(mode, 0o200);
    let ux = xb(mode, 0o100);

    let gr = r(mode, 0o040);
    let gw = w(mode, 0o020);
    let gx = xb(mode, 0o010);

    let or = r(mode, 0o004);
    let ow = w(mode, 0o002);
    let ox = xb(mode, 0o001);

    s.push(ur);
    s.push(uw);
    s.push(if mode & 0o4000 != 0 {
        if ux {
            's'
        } else {
            'S'
        }
    } else if ux {
        'x'
    } else {
        '-'
    });

    s.push(gr);
    s.push(gw);
    s.push(if mode & 0o2000 != 0 {
        if gx {
            's'
        } else {
            'S'
        }
    } else if gx {
        'x'
    } else {
        '-'
    });

    s.push(or);
    s.push(ow);
    s.push(if mode & 0o1000 != 0 {
        if ox {
            't'
        } else {
            'T'
        }
    } else if ox {
        'x'
    } else {
        '-'
    });

    s
}

// ---------------------------------------------------------------------------
// chmod / chown / SELinux file context (Linux xattr)
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn who_clear_mask(who: u8) -> u32 {
    let mut m = 0u32;
    if who & 1 != 0 {
        m |= 0o4700;
    }
    if who & 2 != 0 {
        m |= 0o2070;
    }
    if who & 4 != 0 {
        m |= 0o1007;
    }
    m
}

#[cfg(unix)]
fn parse_perm_bits(who: u8, perm: &str, cur: u32, is_dir: bool) -> CorvoResult<u32> {
    let mut r = false;
    let mut w = false;
    let mut x = false;
    let mut cap_x = false;
    let mut s = false;
    let mut t = false;
    for c in perm.chars() {
        match c {
            'r' => r = true,
            'w' => w = true,
            'x' => x = true,
            'X' => cap_x = true,
            's' => s = true,
            't' => t = true,
            _ => {
                return Err(CorvoError::invalid_argument(format!(
                    "fs.chmod: invalid permission character '{c}'"
                )));
            }
        }
    }
    let any_exec = is_dir || (cur & 0o111) != 0;
    let x_eff = x || (cap_x && any_exec);
    let mut bits = 0u32;
    if who & 1 != 0 {
        if r {
            bits |= 0o400;
        }
        if w {
            bits |= 0o200;
        }
        if x_eff {
            bits |= 0o100;
        }
        if s {
            bits |= 0o4000;
        }
    }
    if who & 2 != 0 {
        if r {
            bits |= 0o040;
        }
        if w {
            bits |= 0o020;
        }
        if x_eff {
            bits |= 0o010;
        }
        if s {
            bits |= 0o2000;
        }
    }
    if who & 4 != 0 {
        if r {
            bits |= 0o004;
        }
        if w {
            bits |= 0o002;
        }
        if x_eff {
            bits |= 0o001;
        }
        if t {
            bits |= 0o1000;
        }
    }
    Ok(bits)
}

#[cfg(unix)]
fn apply_chmod_clause(mode: u32, is_dir: bool, clause: &str) -> CorvoResult<u32> {
    let bytes = clause.as_bytes();
    let mut i = 0usize;
    let mut who = 0u8;
    if i < bytes.len() && matches!(bytes[i], b'+' | b'-' | b'=') {
        who = 7;
    } else {
        while i < bytes.len() {
            match bytes[i] {
                b'u' => who |= 1,
                b'g' => who |= 2,
                b'o' => who |= 4,
                b'a' => who |= 7,
                b'+' | b'-' | b'=' => break,
                _ => {
                    return Err(CorvoError::invalid_argument(format!(
                        "fs.chmod: invalid symbolic clause '{clause}'"
                    )));
                }
            }
            i += 1;
        }
        if who == 0 {
            who = 7;
        }
    }
    if i >= bytes.len() {
        return Err(CorvoError::invalid_argument(
            "fs.chmod: symbolic clause missing operator",
        ));
    }
    let op = bytes[i];
    i += 1;
    let perm_str = std::str::from_utf8(&bytes[i..])
        .map_err(|_| CorvoError::invalid_argument("fs.chmod: invalid UTF-8 in symbolic mode"))?;
    let perm_bits = parse_perm_bits(who, perm_str, mode, is_dir)?;
    let clear = who_clear_mask(who);
    let touch = clear;
    Ok(match op {
        b'+' => mode | (perm_bits & touch),
        b'-' => mode & !(perm_bits & touch),
        b'=' => (mode & !clear) | perm_bits,
        _ => {
            return Err(CorvoError::invalid_argument(
                "fs.chmod: expected '+', '-', or '=' in symbolic mode",
            ));
        }
    })
}

#[cfg(unix)]
fn chmod_apply_mode(path: &Path, mode: u32) -> CorvoResult<()> {
    let mut perms = fs::symlink_metadata(path)
        .map_err(|e| CorvoError::file_system(e.to_string()))?
        .permissions();
    perms.set_mode(mode & 0o7777);
    fs::set_permissions(path, perms).map_err(|e| CorvoError::file_system(e.to_string()))?;
    Ok(())
}

#[cfg(unix)]
fn chmod_apply_spec(path: &Path, spec: &str) -> CorvoResult<()> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(CorvoError::invalid_argument("fs.chmod: empty MODE"));
    }
    if spec.chars().all(|c| matches!(c, '0'..='7')) {
        let mode = u32::from_str_radix(spec, 8).map_err(|_| {
            CorvoError::invalid_argument(format!("fs.chmod: invalid octal mode '{spec}'"))
        })?;
        chmod_apply_mode(path, mode)?;
        return Ok(());
    }
    let meta = fs::symlink_metadata(path).map_err(|e| CorvoError::file_system(e.to_string()))?;
    let mut mode = meta.mode() & 0o7777;
    let is_dir = meta.is_dir();
    for clause in spec.split(',') {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        mode = apply_chmod_clause(mode, is_dir, clause)?;
    }
    chmod_apply_mode(path, mode)?;
    Ok(())
}

#[cfg(unix)]
fn chmod_visit(path: &Path, spec_or_mode: &ChmodArg<'_>) -> CorvoResult<()> {
    match spec_or_mode {
        ChmodArg::Numeric(m) => chmod_apply_mode(path, *m)?,
        ChmodArg::Symbolic(s) => chmod_apply_spec(path, s)?,
    }
    if path.is_dir() {
        let rd = fs::read_dir(path).map_err(|e| CorvoError::file_system(e.to_string()))?;
        for ent in rd {
            let ent = ent.map_err(|e| CorvoError::file_system(e.to_string()))?;
            chmod_visit(&ent.path(), spec_or_mode)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
enum ChmodArg<'a> {
    Numeric(u32),
    Symbolic(&'a str),
}

/// Change file mode bits. `mode` is either a numeric value (same encoding as `st_mode & 07777`)
/// or an octal / symbolic MODE string (e.g. `"755"`, `"u+x"`).
///
/// Args: `path`, `mode` (number or string), `recursive` (bool, default false).
pub fn chmod(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    #[cfg(not(unix))]
    {
        let _ = args;
        return Err(CorvoError::runtime("fs.chmod is only supported on Unix"));
    }
    #[cfg(unix)]
    {
        let path = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("fs.chmod requires a path"))?;
        let mode_val = args.get(1).ok_or_else(|| {
            CorvoError::invalid_argument("fs.chmod requires a mode (number or string)")
        })?;
        let recursive = args.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
        let p = Path::new(path.as_str());
        match mode_val {
            Value::Number(n) => {
                let m = *n as u32;
                if recursive {
                    chmod_visit(p, &ChmodArg::Numeric(m))?;
                } else {
                    chmod_apply_mode(p, m)?;
                }
            }
            Value::String(s) => {
                if recursive {
                    chmod_visit(p, &ChmodArg::Symbolic(s.as_str()))?;
                } else {
                    chmod_apply_spec(p, s.as_str())?;
                }
            }
            _ => {
                return Err(CorvoError::invalid_argument(
                    "fs.chmod: mode must be a number or string",
                ));
            }
        }
        Ok(Value::Boolean(true))
    }
}

/// Change owner and group. Use uid or gid `-1` (number) to leave that id unchanged.
///
/// Args: `path`, `uid`, `gid`, `follow_symlinks` (bool, default true).
pub fn chown(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    #[cfg(not(unix))]
    {
        let _ = args;
        return Err(CorvoError::runtime("fs.chown is only supported on Unix"));
    }
    #[cfg(unix)]
    {
        let path = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("fs.chown requires a path"))?;
        let uid_v = args
            .get(1)
            .and_then(|v| v.as_number())
            .ok_or_else(|| CorvoError::invalid_argument("fs.chown requires uid (number)"))?;
        let gid_v = args
            .get(2)
            .and_then(|v| v.as_number())
            .ok_or_else(|| CorvoError::invalid_argument("fs.chown requires gid (number)"))?;
        let follow = args.get(3).and_then(|v| v.as_bool()).unwrap_or(true);
        let uid = if uid_v < 0.0 {
            None
        } else {
            Some(uid_v as u32)
        };
        let gid = if gid_v < 0.0 {
            None
        } else {
            Some(gid_v as u32)
        };
        let p = Path::new(path.as_str());
        let r = if follow {
            unix_chown(p, uid, gid)
        } else {
            lchown(p, uid, gid)
        };
        r.map_err(|e| CorvoError::file_system(e.to_string()))?;
        Ok(Value::Boolean(true))
    }
}

#[cfg(target_os = "linux")]
pub fn selinux_context_get(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    use std::ffi::CString;

    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.selinux_context_get requires a path"))?;
    let cpath = CString::new(path.as_str())
        .map_err(|_| CorvoError::invalid_argument("fs.selinux_context_get: path contains NUL"))?;
    let cname = CString::new("security.selinux").unwrap();
    // SAFETY: libc getxattr with valid C strings.
    let sz = unsafe { libc::getxattr(cpath.as_ptr(), cname.as_ptr(), std::ptr::null_mut(), 0) };
    if sz < 0 {
        return Err(CorvoError::file_system(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    let mut buf = vec![0u8; sz as usize];
    let sz2 = unsafe {
        libc::getxattr(
            cpath.as_ptr(),
            cname.as_ptr(),
            buf.as_mut_ptr().cast::<libc::c_void>(),
            buf.len(),
        )
    };
    if sz2 < 0 {
        return Err(CorvoError::file_system(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    while buf.last().copied() == Some(0) {
        buf.pop();
    }
    let s = String::from_utf8_lossy(&buf).to_string();
    Ok(Value::String(s))
}

#[cfg(not(target_os = "linux"))]
pub fn selinux_context_get(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    let _ = args;
    Err(CorvoError::runtime(
        "fs.selinux_context_get is only supported on Linux",
    ))
}

#[cfg(target_os = "linux")]
pub fn selinux_context_set(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    use std::ffi::CString;

    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.selinux_context_set requires a path"))?;
    let ctx = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("fs.selinux_context_set requires context"))?;
    let cpath = CString::new(path.as_str())
        .map_err(|_| CorvoError::invalid_argument("fs.selinux_context_set: path contains NUL"))?;
    let cname = CString::new("security.selinux").unwrap();
    let mut val = ctx.as_bytes().to_vec();
    if !val.ends_with(&[0]) {
        val.push(0);
    }
    let r = unsafe {
        libc::setxattr(
            cpath.as_ptr(),
            cname.as_ptr(),
            val.as_ptr().cast::<libc::c_void>(),
            val.len(),
            0,
        )
    };
    if r != 0 {
        return Err(CorvoError::file_system(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    Ok(Value::Boolean(true))
}

#[cfg(not(target_os = "linux"))]
pub fn selinux_context_set(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    let _ = args;
    Err(CorvoError::runtime(
        "fs.selinux_context_set is only supported on Linux",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_write_and_read() {
        let dir = std::env::temp_dir().join("corvo_test_write");
        let path = dir.to_string_lossy().to_string();

        let _ = fs::remove_file(&path);

        let write_args = vec![
            Value::String(path.clone()),
            Value::String("hello world".to_string()),
        ];
        assert_eq!(
            write(&write_args, &empty_args()).unwrap(),
            Value::Boolean(true)
        );

        let read_args = vec![Value::String(path.clone())];
        assert_eq!(
            read(&read_args, &empty_args()).unwrap(),
            Value::String("hello world".to_string())
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_not_found() {
        let args = vec![Value::String("/nonexistent/path/file.txt".to_string())];
        assert!(read(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_exists_true() {
        let tmp = std::env::temp_dir();
        let args = vec![Value::String(tmp.to_string_lossy().to_string())];
        assert_eq!(exists(&args, &empty_args()).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_exists_false() {
        let args = vec![Value::String("/nonexistent/path".to_string())];
        assert_eq!(exists(&args, &empty_args()).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_mkdir_and_list_dir() {
        let dir = std::env::temp_dir().join("corvo_test_dir");
        let path = dir.to_string_lossy().to_string();
        let _ = fs::remove_dir_all(&path);

        let mkdir_args = vec![Value::String(path.clone()), Value::Boolean(true)];
        assert_eq!(
            mkdir(&mkdir_args, &empty_args()).unwrap(),
            Value::Boolean(true)
        );

        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn test_write_no_args() {
        assert!(write(&[], &empty_args()).is_err());
    }

    #[test]
    fn test_exists_no_args() {
        assert!(exists(&[], &empty_args()).is_err());
    }

    #[test]
    fn test_delete_no_args() {
        assert!(delete(&[], &empty_args()).is_err());
    }

    #[test]
    fn test_stat_directory() {
        let tmp = std::env::temp_dir();
        let args = vec![Value::String(tmp.to_string_lossy().to_string())];
        let result = stat(&args, &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert!(m.contains_key("size"));
                assert!(m.contains_key("is_dir"));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_path_parent_dot_is_parent_of_cwd() {
        let expected = std::env::current_dir()
            .ok()
            .and_then(|c| c.parent().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_default();
        let args = vec![Value::String(".".to_string())];
        assert_eq!(
            path_parent(&args, &empty_args()).unwrap(),
            Value::String(expected)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_chmod_octal_and_symbolic() {
        let file = std::env::temp_dir().join("corvo_test_chmod_file");
        let path = file.to_string_lossy().to_string();
        let _ = fs::remove_file(&path);
        fs::write(&path, b"x").unwrap();

        let chmod_args = vec![
            Value::String(path.clone()),
            Value::String("600".to_string()),
            Value::Boolean(false),
        ];
        assert_eq!(
            chmod(&chmod_args, &empty_args()).unwrap(),
            Value::Boolean(true)
        );
        let mode = fs::symlink_metadata(&path).unwrap().mode() & 0o7777;
        assert_eq!(mode, 0o600);

        let chmod_sym = vec![
            Value::String(path.clone()),
            Value::String("u+x".to_string()),
            Value::Boolean(false),
        ];
        assert_eq!(
            chmod(&chmod_sym, &empty_args()).unwrap(),
            Value::Boolean(true)
        );
        let mode2 = fs::symlink_metadata(&path).unwrap().mode() & 0o7777;
        assert_eq!(mode2, 0o700);

        let _ = fs::remove_file(&path);
    }
}
