# Corvo Standard Library Cheatsheet

| Namespace/Method | Parameters | Return Value | Description | Example File |
| --- | --- | --- | --- | --- |
| `sys.echo` | `args...` | `Null` | Prints arguments to stdout with a newline | `examples/sys_example.corvo` |
| `sys.printf` | `format, args...` | `Null` | Prints formatted string | `examples/sys_example.corvo` |
| `sys.print` | `args...` | `Null` | Prints arguments without a newline | `examples/sys_example.corvo` |
| `sys.eprint` | `args...` | `Null` | Prints arguments to stderr with a newline | `examples/sys_example.corvo` |
| `sys.read_line` | `[prompt]` | `String` | Reads a line from stdin | `examples/sys_example.corvo` |
| `sys.sleep` | `ms` | `Null` | Pauses execution for specified milliseconds | `examples/sys_example.corvo` |
| `sys.panic` | `[message]` | `Never` | Aborts execution with an error | `examples/error_handling.corvo` |
| `sys.exit` | `[code]` | `Never` | Exits the process with status code | `examples/sys_example.corvo` |
| `sys.exec` | `cmd_list, [kwargs]` | `Map` | Executes an external command | `examples/sys_example.corvo` |
| `sys.read_all` | `` | `String` | Reads all input from stdin until EOF | `examples/sys_example.corvo` |
| `sys.chroot` | `path` | `Boolean` | Changes root directory | `examples/sys_example.corvo` |
| `sys.nice` | `inc` | `Boolean` | Changes process priority | `examples/sys_example.corvo` |
| `sys.sync` | `` | `Boolean` | Flushes file system buffers | `examples/sys_example.corvo` |
| `sys.stdin_isatty` | `` | `Boolean` | Checks if stdin is a terminal | `examples/sys_example.corvo` |
| `sys.stdout_isatty` | `` | `Boolean` | Checks if stdout is a terminal | `examples/sys_example.corvo` |
| `os.get_env` | `name` | `String` | Gets environment variable | `examples/os_example.corvo` |
| `os.set_env` | `name, value` | `Boolean` | Sets environment variable | `examples/os_example.corvo` |
| `os.exec` | `cmd_list` | `Map` | Alias for sys.exec | `examples/os_example.corvo` |
| `os.info` | `` | `Map` | Gets OS information | `examples/os_example.corvo` |
| `os.environ` | `` | `Map` | Gets all environment variables | `examples/os_example.corvo` |
| `os.groups` | `` | `List` | Gets current user groups | `examples/os_example.corvo` |
| `os.hostid` | `` | `String` | Gets host ID | `examples/os_example.corvo` |
| `os.nproc` | `` | `Number` | Gets number of processors | `examples/os_example.corvo` |
| `os.df` | `[path]` | `Map` | Gets disk free space | `examples/os_example.corvo` |
| `os.argv` | `` | `List` | Gets command line arguments | `examples/os_example.corvo` |
| `os.getcwd` | `` | `String` | Gets current working directory | `examples/os_example.corvo` |
| `os.username` | `` | `String` | Gets current username | `examples/os_example.corvo` |
| `os.ttyname` | `` | `String` | Gets TTY name | `examples/os_example.corvo` |
| `os.uptime` | `` | `Number` | Gets system uptime | `examples/os_example.corvo` |
| `os.load_average` | `` | `List` | Gets system load averages | `examples/os_example.corvo` |
| `os.user_count` | `` | `Number` | Gets number of logged-in users | `examples/os_example.corvo` |
| `os.users` | `` | `List` | Gets logged in users | `examples/os_example.corvo` |
| `os.user_id` | `` | `Number` | Gets effective user ID | `examples/os_example.corvo` |
| `os.group_id` | `` | `Number` | Gets effective group ID | `examples/os_example.corvo` |
| `os.tty_get_mode` | `` | `Map` | Gets TTY mode | `examples/os_example.corvo` |
| `os.tty_set_mode` | `mode` | `Boolean` | Sets TTY mode | `examples/os_example.corvo` |
| `os.temp_dir` | `` | `String` | Gets system temp directory | `examples/os_example.corvo` |
| `math.add` | `a, b` | `Number` | Addition | `examples/math_example.corvo` |
| `math.sub` | `a, b` | `Number` | Subtraction | `examples/math_example.corvo` |
| `math.mul` | `a, b` | `Number` | Multiplication | `examples/math_example.corvo` |
| `math.div` | `a, b` | `Number` | Division | `examples/math_example.corvo` |
| `math.mod` | `a, b` | `Number` | Modulo | `examples/math_example.corvo` |
| `math.max` | `args...` | `Number` | Maximum value | `examples/math_example.corvo` |
| `math.min` | `args...` | `Number` | Minimum value | `examples/math_example.corvo` |
| `math.floor` | `a` | `Number` | Rounds down | `examples/math_example.corvo` |
| `math.round` | `a` | `Number` | Rounds to nearest integer | `examples/math_example.corvo` |
| `math.ceil` | `a` | `Number` | Rounds up | `examples/math_example.corvo` |
| `math.random` | `[min, max]` | `Number` | Generates random number | `examples/math_example.corvo` |
| `math.human_bytes` | `bytes` | `String` | Formats bytes human readable | `examples/math_example.corvo` |
| `math.parse_size` | `str` | `Number` | Parses human readable size | `examples/math_example.corvo` |
| `math.range` | `start, end` | `List` | Generates range list | `examples/math_example.corvo` |
| `fs.read` | `path` | `String` | Reads file contents | `examples/fs_example.corvo` |
| `fs.read_lines` | `path` | `List` | Reads file lines | `examples/fs_example.corvo` |
| `fs.write` | `path, content, [follow_symlinks]` | `Boolean` | Writes to file; optional `follow_symlinks` defaults to `true`; on Unix, `false` opens destination with `O_NOFOLLOW` (errors on symlink); on non-Unix, `false` is rejected | `examples/fs_example.corvo` |
| `fs.append` | `path, content` | `Boolean` | Appends to file | `examples/fs_example.corvo` |
| `fs.delete` | `path` | `Boolean` | Deletes file or directory | `examples/fs_example.corvo` |
| `fs.exists` | `path` | `Boolean` | Checks if path exists | `examples/fs_example.corvo` |
| `fs.mkdir` | `path, [recursive], [mode]` | `Boolean` | Creates directory; optional Unix third argument must be numeric (integer 0–4095); bits like `st_mode & 07777`, passed to `mkdir(2)` via `DirBuilder::mode`, so effective permissions follow POSIX (`mode & ~umask`); use `fs.chmod` after creation if you need exact bits independent of umask | `examples/fs_example.corvo` |
| `fs.mkfifo` | `path` | `Boolean` | Creates FIFO special file | `examples/fs_example.corvo` |
| `fs.mknod` | `path, type` | `Boolean` | Creates block/char device | `examples/fs_example.corvo` |
| `fs.list_dir` | `path` | `List` | Lists directory contents | `examples/fs_example.corvo` |
| `fs.copy` | `src, dst, [follow_symlinks]` | `Boolean` | Copies file; optional `follow_symlinks` defaults to `true`; on Unix, `false` preserves a symlink **source** (creating a new symlink at dest, replacing an existing dest file if needed), rejects a symlink **destination** and special source nodes (char/block/FIFO/socket); on non-Unix, `false` is rejected | `examples/fs_example.corvo` |
| `fs.move` | `src, dst` | `Boolean` | Moves file | `examples/fs_example.corvo` |
| `fs.link` | `src, dst` | `Boolean` | Creates hard link | `examples/fs_example.corvo` |
| `fs.symlink` | `src, dst` | `Boolean` | Creates symlink | `examples/fs_example.corvo` |
| `fs.realpath` | `path` | `String` | Resolves absolute path | `examples/fs_example.corvo` |
| `fs.truncate` | `path, size` | `Boolean` | Truncates file | `examples/fs_example.corvo` |
| `fs.touch` | `path, [follow_symlinks]` | `Boolean` | Creates file if missing and bumps atime/mtime to now via `futimens(2)` on Unix (read-only open for existing files, then times); optional `follow_symlinks` defaults to `true`; on Unix, `false` uses `O_NOFOLLOW` and rejects symlink destinations; on non-Unix, `false` is rejected | `examples/fs_example.corvo` |
| `fs.stat` | `path` | `Map` | Gets file metadata | `examples/fs_example.corvo` |
| `fs.read_link` | `path` | `String` | Reads symlink target | `examples/fs_example.corvo` |
| `fs.read_dir_meta` | `path` | `List` | Lists dir with metadata | `examples/fs_example.corvo` |
| `fs.mktemp` | `[template], [is_dir], [tmp_dir], [suffix]` | `String` | Creates temporary file/dir; on Unix, file mode is forced to `0600` (`create_new`) | `examples/fs_example.corvo` |
| `fs.read_hex` | `path` | `String` | Reads file as hex | `examples/fs_example.corvo` |
| `fs.write_hex` | `path, hex` | `Boolean` | Writes hex to file | `examples/fs_example.corvo` |
| `fs.read_meta` | `path` | `Map` | Gets detailed file metadata | `examples/fs_example.corvo` |
| `fs.path_filename` | `path` | `String` | Gets file name from path | `examples/fs_example.corvo` |
| `fs.path_parent` | `path` | `String` | Gets parent directory | `examples/fs_example.corvo` |
| `fs.path_join` | `args...` | `String` | Joins path segments | `examples/fs_example.corvo` |
| `fs.path_relative` | `base, path` | `String` | Gets relative path | `examples/fs_example.corvo` |
| `fs.chmod` | `path, mode` | `Boolean` | Changes file permissions | `examples/fs_example.corvo` |
| `fs.chown` | `path, uid, gid` | `Boolean` | Changes file owner | `examples/fs_example.corvo` |
| `fs.selinux_context_get` | `path` | `String` | Gets SELinux context | `examples/fs_example.corvo` |
| `fs.selinux_context_set` | `path, ctx` | `Boolean` | Sets SELinux context | `examples/fs_example.corvo` |
| `http.get` | `url, [headers]` | `Map` | HTTP GET request | `examples/http_example.corvo` |
| `http.post` | `url, body, [headers]` | `Map` | HTTP POST request | `examples/http_example.corvo` |
| `http.put` | `url, body, [headers]` | `Map` | HTTP PUT request | `examples/http_example.corvo` |
| `http.delete` | `url, [headers]` | `Map` | HTTP DELETE request | `examples/http_example.corvo` |
| `net.tcp_listen` | `addr` | `Number` | Starts TCP listener | `examples/net_tcp.corvo` |
| `net.tcp_accept` | `listener` | `Number` | Accepts TCP connection | `examples/net_tcp.corvo` |
| `net.tcp_close_listener` | `listener` | `Boolean` | Closes listener | `examples/net_tcp.corvo` |
| `net.tcp_connect` | `addr` | `Number` | Connects to TCP server | `examples/net_tcp.corvo` |
| `net.tcp_read` | `conn` | `String` | Reads from TCP connection | `examples/net_tcp.corvo` |
| `net.tcp_write` | `conn, data` | `Boolean` | Writes to TCP connection | `examples/net_tcp.corvo` |
| `net.tcp_close` | `conn` | `Boolean` | Closes TCP connection | `examples/net_tcp.corvo` |
| `dns.resolve` | `domain` | `List` | Resolves A records | `examples/dns_example.corvo` |
| `dns.lookup` | `domain` | `List` | Performs full DNS lookup | `examples/dns_example.corvo` |
| `crypto.hash` | `algo, data` | `String` | Hashes string | `examples/crypto_example.corvo` |
| `crypto.hash_file` | `algo, path` | `String` | Hashes file | `examples/crypto_example.corvo` |
| `crypto.hash_stdin` | `algo` | `String` | Hashes stdin | `examples/crypto_example.corvo` |
| `crypto.checksum` | `algo, data` | `String` | Calculates checksum | `examples/crypto_example.corvo` |
| `crypto.crc32_file` | `path` | `String` | CRC32 of file | `examples/crypto_example.corvo` |
| `crypto.crc32_stdin` | `` | `String` | CRC32 of stdin | `examples/crypto_example.corvo` |
| `crypto.encrypt` | `algo, key, data` | `String` | Encrypts data | `examples/crypto_example.corvo` |
| `crypto.decrypt` | `algo, key, data` | `String` | Decrypts data | `examples/crypto_example.corvo` |
| `crypto.uuid` | `` | `String` | Generates UUID v4 | `examples/crypto_example.corvo` |
| `json.parse` | `str` | `Value` | Parses JSON string | `examples/json_example.corvo` |
| `json.stringify` | `val` | `String` | Serializes to JSON | `examples/json_example.corvo` |
| `yaml.parse` | `str` | `Value` | Parses YAML string | `examples/yaml_example.corvo` |
| `yaml.stringify` | `val` | `String` | Serializes to YAML | `examples/yaml_example.corvo` |
| `hcl.parse` | `str` | `Value` | Parses HCL string | `examples/hcl_example.corvo` |
| `hcl.stringify` | `val` | `String` | Serializes to HCL | `examples/hcl_example.corvo` |
| `csv.parse` | `str` | `List` | Parses CSV string | `examples/csv_example.corvo` |
| `db.connect` | `url, [max_conn]` | `DatabasePool` | Connects to SQLite (`sqlite:`) or Postgres (`postgres://`, `postgresql://`) | `examples/db_example.corvo` |
| `db.query` | `pool, sql, args...`| `List` | Executes a SQL query returning rows | `examples/db_example.corvo` |
| `db.execute` | `pool, sql, args...`| `Number` | Executes a SQL statement | `examples/db_example.corvo` |
| `db.close` | `pool` | `Null` | Closes the database pool | `examples/db_example.corvo` |
| `amqp.connect` | `url` | `AmqpConnection` | Connects to an AMQP broker | `examples/amqp_example.corvo` |
| `amqp.publish` | `conn, exchange, routing_key, payload`| `Boolean` | Publishes a message | `examples/amqp_example.corvo` |
| `amqp.queue_delete` | `conn, queue_name` | `Number` | Deletes a queue | `examples/amqp_example.corvo` |
| `amqp.queue_purge` | `conn, queue_name` | `Number` | Purges a queue | `examples/amqp_example.corvo` |
| `xml.parse` | `str` | `Value` | Parses XML string | `examples/xml_example.corvo` |
| `env.parse` | `str` | `Map` | Parses dotenv string | `examples/env_example.corvo` |
| `args.scan` | `` | `Map` | Scans command line flags | `examples/args.corvo` |
| `args.parse` | `spec` | `Map` | Parses arguments by spec | `examples/args_parse.corvo` |
| `time.format_local` | `fmt` | `String` | Formats local time | `examples/time_example.corvo` |
| `time.format_utc` | `fmt` | `String` | Formats UTC time | `examples/time_example.corvo` |
| `time.unix_now` | `` | `Number` | Unix timestamp (seconds) | `examples/time_example.corvo` |
| `time.parse_date` | `date, fmt` | `Number` | Parses date string | `examples/time_example.corvo` |
| `time.boot_time` | `` | `Number` | System boot timestamp | `examples/time_example.corvo` |
| `template.render` | `tmpl, ctx` | `String` | Renders template string | `examples/template_example.corvo` |
| `template.render_file` | `path, ctx` | `String` | Renders template file | `examples/template_example.corvo` |
| `llm.model` | `name` | `String` | Sets LLM model | `examples/llm_example.corvo` |
| `llm.prompt` | `prompt` | `String` | Sends LLM prompt | `examples/llm_example.corvo` |
| `llm.embed` | `text` | `List` | Gets embeddings | `examples/llm_example.corvo` |
| `llm.chat` | `messages` | `String` | Chat completion | `examples/llm_example.corvo` |
| `notifications.smtp` | `opts` | `Boolean` | Sends email | `examples/notifications_example.corvo` |
| `notifications.slack` | `opts` | `Boolean` | Slack message | `examples/notifications_example.corvo` |
| `notifications.telegram` | `opts` | `Boolean` | Telegram message | `examples/notifications_example.corvo` |
| `notifications.mattermost` | `opts` | `Boolean` | Mattermost message | `examples/notifications_example.corvo` |
| `notifications.gitter` | `opts` | `Boolean` | Gitter message | `examples/notifications_example.corvo` |
| `notifications.messenger` | `opts` | `Boolean` | Messenger message | `examples/notifications_example.corvo` |
| `notifications.discord` | `opts` | `Boolean` | Discord message | `examples/notifications_example.corvo` |
| `notifications.teams` | `opts` | `Boolean` | Teams message | `examples/notifications_example.corvo` |
| `notifications.x` | `opts` | `Boolean` | X/Twitter post | `examples/notifications_example.corvo` |
| `notifications.os` | `opts` | `Boolean` | Desktop notification | `examples/notifications_example.corvo` |
| `notifications.irc` | `opts` | `Boolean` | IRC message | `examples/notifications_example.corvo` |
| `re.match` | `pattern, str` | `Boolean` | Regex exact match | `examples/regex.corvo` |
| `re.find` | `pattern, str` | `String` | Finds first regex match | `examples/regex.corvo` |
| `re.find_all` | `pattern, str` | `List` | Finds all regex matches | `examples/regex.corvo` |
| `re.replace` | `pattern, str, repl` | `String` | Replaces first match | `examples/regex.corvo` |
| `re.replace_all` | `pattern, str, repl` | `String` | Replaces all matches | `examples/regex.corvo` |
| `re.split` | `pattern, str` | `List` | Splits string by regex | `examples/regex.corvo` |
| `re.new` | `pattern` | `Regex` | Compiles regex object | `examples/regex.corvo` |
| `re.posix_class_chars` | `class` | `String` | Expands POSIX class to ASCII set (`graph`,`print`,`space`,`upper`,`lower`) with POSIX-consistent definitions | `examples/regex.corvo` |
| `re.posix_class_translate` | `text, from_class, to_class` | `String` | Translates chars by POSIX classes (e.g. `upper -> lower`), reusing last destination char when destination set is shorter | `examples/regex.corvo` |
| `string.concat` | `args...` | `String` | Concatenates strings | `examples/string_methods.corvo` |
| `string.replace` | `s, old, new` | `String` | Replaces substring | `examples/string_methods.corvo` |
| `string.split` | `s, delim` | `List` | Splits string | `examples/string_methods.corvo` |
| `string.trim` | `s` | `String` | Trims whitespace | `examples/string_methods.corvo` |
| `string.trim_start` | `s` | `String` | Trims leading whitespace | `examples/string_methods.corvo` |
| `string.trim_end` | `s` | `String` | Trims trailing whitespace | `examples/string_methods.corvo` |
| `string.contains` | `s, sub` | `Boolean` | Checks for substring | `examples/string_methods.corvo` |
| `string.starts_with` | `s, sub` | `Boolean` | Checks prefix | `examples/string_methods.corvo` |
| `string.ends_with` | `s, sub` | `Boolean` | Checks suffix | `examples/string_methods.corvo` |
| `string.to_lower` | `s` | `String` | Converts to lowercase | `examples/string_methods.corvo` |
| `string.to_upper` | `s` | `String` | Converts to uppercase | `examples/string_methods.corvo` |
| `string.len` | `s` | `Number` | Gets string length | `examples/string_methods.corvo` |
| `string.reverse` | `s` | `String` | Reverses string | `examples/string_methods.corvo` |
| `string.is_empty` | `s` | `Boolean` | Checks if string is empty | `examples/string_methods.corvo` |
| `string.pad_start` | `s, len, [c]` | `String` | Pads string start | `examples/string_methods.corvo` |
| `string.pad_end` | `s, len, [c]` | `String` | Pads string end | `examples/string_methods.corvo` |
| `string.fnmatch` | `s, pat` | `Boolean` | Matches glob pattern | `examples/string_methods.corvo` |
| `string.byte_slice` | `s, start, end` | `String` | Slices by bytes | `examples/string_methods.corvo` |
| `string.substring` | `s, start, end` | `String` | Slices by characters | `examples/string_methods.corvo` |
| `string.index_of` | `s, sub` | `Number` | Finds index of substring | `examples/string_methods.corvo` |
| `string.last_index_of` | `s, sub` | `Number` | Finds last index of substring | `examples/string_methods.corvo` |
| `string.char_at` | `s, idx` | `String` | Gets char at index | `examples/string_methods.corvo` |
| `string.repeat` | `s, n` | `String` | Repeats string n times | `examples/string_methods.corvo` |
| `string.replace_first` | `s, old, new` | `String` | Replaces first occurrence | `examples/string_methods.corvo` |
| `string.count` | `s, sub` | `Number` | Counts occurrences | `examples/string_methods.corvo` |
| `string.chars` | `s` | `List` | Gets list of characters | `examples/string_methods.corvo` |
| `string.base64_encode` | `s` | `String` | Base64 encodes string | `examples/string_methods.corvo` |
| `string.base64_decode` | `s` | `String` | Base64 decodes string | `examples/string_methods.corvo` |
| `string.base32_encode` | `s` | `String` | Base32 encodes string | `examples/string_methods.corvo` |
| `string.base32_decode` | `s` | `String` | Base32 decodes string | `examples/string_methods.corvo` |
| `string.base32hex_encode` | `s` | `String` | Base32hex encodes string | `examples/string_methods.corvo` |
| `string.base32hex_decode` | `s` | `String` | Base32hex decodes string | `examples/string_methods.corvo` |
| `string.hex_encode` | `s` | `String` | Hex encodes string | `examples/string_methods.corvo` |
| `string.hex_decode` | `s` | `String` | Hex decodes string | `examples/string_methods.corvo` |
| `number.to_string` | `n` | `String` | Converts to string | `examples/number_methods.corvo` |
| `number.parse` | `s` | `Number` | Parses from string | `examples/number_methods.corvo` |
| `number.is_nan` | `n` | `Boolean` | Checks if NaN | `examples/number_methods.corvo` |
| `number.is_infinite` | `n` | `Boolean` | Checks if Infinite | `examples/number_methods.corvo` |
| `number.is_finite` | `n` | `Boolean` | Checks if finite | `examples/number_methods.corvo` |
| `number.abs` | `n` | `Number` | Absolute value | `examples/number_methods.corvo` |
| `number.floor` | `n` | `Number` | Floor value | `examples/number_methods.corvo` |
| `number.ceil` | `n` | `Number` | Ceil value | `examples/number_methods.corvo` |
| `number.round` | `n` | `Number` | Round value | `examples/number_methods.corvo` |
| `number.sqrt` | `n` | `Number` | Square root | `examples/number_methods.corvo` |
| `number.pow` | `n, p` | `Number` | Power | `examples/number_methods.corvo` |
| `number.min` | `n, args...` | `Number` | Minimum | `examples/number_methods.corvo` |
| `number.max` | `n, args...` | `Number` | Maximum | `examples/number_methods.corvo` |
| `number.clamp` | `n, min, max` | `Number` | Clamps value | `examples/number_methods.corvo` |
| `list.push` | `l, val` | `List` | Appends to list | `examples/list_methods.corvo` |
| `list.pop` | `l` | `Value` | Removes last element | `examples/list_methods.corvo` |
| `list.get` | `l, idx` | `Value` | Gets element by index | `examples/list_methods.corvo` |
| `list.set` | `l, idx, val` | `List` | Sets element by index | `examples/list_methods.corvo` |
| `list.len` | `l` | `Number` | Gets list length | `examples/list_methods.corvo` |
| `list.first` | `l` | `Value` | Gets first element | `examples/list_methods.corvo` |
| `list.last` | `l` | `Value` | Gets last element | `examples/list_methods.corvo` |
| `list.concat` | `l1, l2` | `List` | Concatenates lists | `examples/list_methods.corvo` |
| `list.is_empty` | `l` | `Boolean` | Checks if empty | `examples/list_methods.corvo` |
| `list.contains` | `l, val` | `Boolean` | Checks if contains | `examples/list_methods.corvo` |
| `list.delete` | `l, idx` | `List` | Removes element by index | `examples/list_methods.corvo` |
| `list.filter` | `l, proc` | `List` | Filters list | `examples/list_methods.corvo` |
| `list.map` | `l, proc` | `List` | Maps list | `examples/list_methods.corvo` |
| `list.reduce` | `l, proc, [init]` | `Value` | Reduces list | `examples/list_methods.corvo` |
| `list.find` | `l, proc` | `Value` | Finds element | `examples/list_methods.corvo` |
| `list.sort` | `l` | `List` | Sorts list | `examples/list_methods.corvo` |
| `list.sort_version` | `l` | `List` | Sorts as versions | `examples/list_methods.corvo` |
| `list.sort_maps_by_key` | `l, key` | `List` | Sorts list of maps | `examples/list_methods.corvo` |
| `list.columnate` | `l` | `String` | Formats into columns | `examples/list_methods.corvo` |
| `list.reverse` | `l` | `List` | Reverses list | `examples/list_methods.corvo` |
| `list.flatten` | `l` | `List` | Flattens nested lists | `examples/list_methods.corvo` |
| `list.unique` | `l` | `List` | Removes duplicates | `examples/list_methods.corvo` |
| `list.join` | `l, sep` | `String` | Joins with separator | `examples/list_methods.corvo` |
| `list.slice` | `l, start, end` | `List` | Slices list | `examples/list_methods.corvo` |
| `list.new` | `args...` | `List` | Creates new list | `examples/new_collections.corvo` |
| `map.get` | `m, key` | `Value` | Gets value by key | `examples/map_methods.corvo` |
| `map.set` | `m, key, val` | `Map` | Sets value by key | `examples/map_methods.corvo` |
| `map.has` | `m, key` | `Boolean` | Alias for has_key | `examples/map_methods.corvo` |
| `map.has_key` | `m, key` | `Boolean` | Checks if key exists | `examples/map_methods.corvo` |
| `map.delete` | `m, key` | `Map` | Removes key | `examples/map_methods.corvo` |
| `map.remove` | `m, key` | `Map` | Alias for delete | `examples/map_methods.corvo` |
| `map.keys` | `m` | `List` | Gets all keys | `examples/map_methods.corvo` |
| `map.values` | `m` | `List` | Gets all values | `examples/map_methods.corvo` |
| `map.entries` | `m` | `List` | Gets [key, value] pairs | `examples/map_methods.corvo` |
| `map.len` | `m` | `Number` | Gets number of keys | `examples/map_methods.corvo` |
| `map.is_empty` | `m` | `Boolean` | Checks if empty | `examples/map_methods.corvo` |
| `map.merge` | `m1, m2` | `Map` | Merges two maps | `examples/map_methods.corvo` |
| `map.new` | `args...` | `Map` | Creates new map | `examples/new_collections.corvo` |
| `map.column` | `m` | `String` | Formats into columns | `examples/map_methods.corvo` |
| `var.get` | `name` | `Value` | Gets dynamic variable | `examples/variables.corvo` |
| `var.set` | `name, value` | `Value` | Sets dynamic variable | `examples/variables.corvo` |
| `static.get` | `name` | `Value` | Gets static variable | `examples/variables.corvo` |
| `static.set` | `name, value` | `Value` | Sets static variable | `examples/variables.corvo` |
| `http_listen` | `addr, @req, @resp` | `Null` | Starts HTTP server | `examples/http_listen.corvo` |

## Oxide Deployment (Lean Binaries)

Corvo supports a specialized transpilation mode called **Oxide** for creating ultra-lean binaries.

| Command | Description |
| --- | --- |
| `corvo --oxide <file.corvo>` | Transpiles to a lean Rust project with static dispatch |
| `corvo --oxide <file> -o <dir>` | Specifies the output directory for the oxide project |

**Optimization Tips:**
- Oxide automatically excludes unused standard library modules.
- Binaries are built with `opt-level = "z"`, `LTO`, and `strip` by default.
- Hello World binaries in Oxide mode are typically **< 400KB**.

## Shorthands

Corvo provides several shorthand operators for common variable assignments:

| Shorthand | Description | Example File |
| --- | --- | --- |
| `@var++` | Increments a number by 1 | `examples/shorthands.corvo` |
| `@var--` | Decrements a number by 1 | `examples/shorthands.corvo` |
| `@var += <number>` | Adds a number to the variable | `examples/shorthands.corvo` |
| `@var -= <number>` | Subtracts a number from the variable | `examples/shorthands.corvo` |
| `@var += "string"` | Concatenates a string to the variable | `examples/shorthands.corvo` |
| `@var -= "string"` | Removes all occurrences of a substring from the variable | `examples/shorthands.corvo` |
| `@var or= (val1, ...)` | Assigns the first truthy value from a list of candidates | `examples/or_assign.corvo` |

## Blocks & Control Flow

Corvo has several built-in block structures for control flow, iteration, and server execution:

| Block | Description | Syntax Example |
| --- | --- | --- |
| `prep` | Compile-time block for defining static variables | `prep { static.set("VERSION", "1.0") }` |
| `try` / `fallback` | Error handling | `try { ... } fallback { ... }` |
| `loop` | Infinite loop, break manually | `loop { ... }` |
| `browse` | Iterates over lists, maps, or strings | `browse(@list, @idx, @val) { ... }` |
| `async_browse` | Parallel iteration over a list with shared variables | `async_browse(@list, @worker, @item, shared @count) { ... }` |
| `if` / `else` | Conditional execution | `if (@cond) { ... } else { ... }` |
| `dont_panic` | Suppresses runtime errors within the block | `dont_panic { sys.panic() }` |
| `http_listen` | Starts a concurrent HTTP server loop | `http_listen("0.0.0.0:8080", @req) { ... }` |
| `amqp_consume` | Starts an AMQP queue consumer | `amqp_consume(@conn, "queue", @msg) { ... }` |
| `procedure` | Defines a reusable function block | `@my_func = procedure(@arg) { ... }` |
