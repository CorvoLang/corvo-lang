pub mod args;
pub mod crypto;
pub mod csv;
pub mod dns;
pub mod env;
pub mod fs;
pub mod hcl;
pub mod http;
pub mod json;
pub mod llm;
pub mod math;
pub mod net;
pub mod notifications;
pub mod os;
pub mod re;
pub mod sys;
pub mod template;
pub mod time;
pub mod xml;
pub mod yaml;

use crate::runtime::RuntimeState;
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn call(
    name: &str,
    args: &[Value],
    named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    match name {
        "sys.echo" => sys::echo(args, named_args),
        "sys.print" => sys::print_no_newline(args, named_args),
        "sys.eprint" => sys::eprint_newline(args, named_args),
        "sys.read_line" => sys::read_line(args, named_args),
        "sys.sleep" => sys::sleep(args, named_args),
        "sys.panic" => sys::panic(args, named_args),
        "sys.exit" => sys::exit_process(args, named_args),
        "sys.exec" => sys::exec(args, named_args),

        "os.get_env" => os::get_env(args, named_args),
        "os.set_env" => os::set_env(args, named_args),
        "os.exec" => os::exec(args, named_args),
        "os.info" => os::info(args, named_args),
        "os.argv" => os::argv(args, named_args, state),
        "os.getcwd" => os::getcwd(args, named_args),
        "os.uptime" => os::uptime(args, named_args),
        "os.load_average" => os::load_average(args, named_args),
        "os.user_count" => os::user_count(args, named_args),
        "os.user_id" => os::user_id(args, named_args),
        "os.group_id" => os::group_id(args, named_args),

        "args.scan" => args::scan(args, named_args),
        "args.parse" => args::parse(args, named_args),

        "math.add" => math::add(args, named_args),
        "math.sub" => math::sub(args, named_args),
        "math.mul" => math::mul(args, named_args),
        "math.div" => math::div(args, named_args),
        "math.mod" => math::modulo(args, named_args),
        "math.max" => math::max(args, named_args),
        "math.human_bytes" => math::human_bytes(args, named_args),

        "fs.read" => fs::read(args, named_args),
        "fs.write" => fs::write(args, named_args),
        "fs.append" => fs::append(args, named_args),
        "fs.delete" => fs::delete(args, named_args),
        "fs.exists" => fs::exists(args, named_args),
        "fs.mkdir" => fs::mkdir(args, named_args),
        "fs.list_dir" => fs::list_dir(args, named_args),
        "fs.copy" => fs::copy(args, named_args),
        "fs.move" => fs::move_file(args, named_args),
        "fs.stat" => fs::stat(args, named_args),
        "fs.read_link" => fs::read_link(args, named_args),
        "fs.read_dir_meta" => fs::read_dir_meta(args, named_args),
        "fs.read_meta" => fs::read_meta(args, named_args),
        "fs.path_parent" => fs::path_parent(args, named_args),
        "fs.path_relative" => fs::path_relative(args, named_args),
        "fs.chmod" => fs::chmod(args, named_args),
        "fs.chown" => fs::chown(args, named_args),
        "fs.selinux_context_get" => fs::selinux_context_get(args, named_args),
        "fs.selinux_context_set" => fs::selinux_context_set(args, named_args),

        "time.format_local" => time::format_local(args, named_args),
        "time.format_utc" => time::format_utc(args, named_args),
        "time.unix_now" => time::unix_now(args, named_args),
        "time.parse_date" => time::parse_date(args, named_args),
        "time.boot_time" => time::boot_time(args, named_args),

        "http.get" => http::get(args, named_args),
        "http.post" => http::post(args, named_args),
        "http.put" => http::put(args, named_args),
        "http.delete" => http::delete(args, named_args),

        "net.tcp_listen" => net::tcp_listen(args, named_args, state),
        "net.tcp_accept" => net::tcp_accept(args, named_args, state),
        "net.tcp_close_listener" => net::tcp_close_listener(args, named_args, state),
        "net.tcp_connect" => net::tcp_connect(args, named_args, state),
        "net.tcp_read" => net::tcp_read(args, named_args, state),
        "net.tcp_write" => net::tcp_write(args, named_args, state),
        "net.tcp_close" => net::tcp_close(args, named_args, state),

        "dns.resolve" => dns::resolve(args, named_args),
        "dns.lookup" => dns::lookup(args, named_args),

        "crypto.hash" => crypto::hash(args, named_args),
        "crypto.hash_file" => crypto::hash_file(args, named_args),
        "crypto.hash_stdin" => crypto::hash_stdin(args, named_args),
        "crypto.checksum" => crypto::checksum(args, named_args),
        "crypto.crc32_file" => crypto::crc32_file(args, named_args),
        "crypto.encrypt" => crypto::encrypt(args, named_args),
        "crypto.decrypt" => crypto::decrypt(args, named_args),
        "crypto.uuid" => crypto::uuid(args, named_args),

        "json.parse" => json::parse_value(args, named_args),
        "json.stringify" => json::stringify(args, named_args),

        "yaml.parse" => yaml::parse_value(args, named_args),
        "yaml.stringify" => yaml::stringify(args, named_args),

        "hcl.parse" => hcl::parse_value(args, named_args),
        "hcl.stringify" => hcl::stringify(args, named_args),

        "csv.parse" => csv::parse_value(args, named_args),

        "xml.parse" => xml::parse_value(args, named_args),

        "env.parse" => env::parse_value(args, named_args),

        "template.render" => template::render(args, named_args),
        "template.render_file" => template::render_file(args, named_args),

        "llm.model" => llm::model(args, named_args),
        "llm.prompt" => llm::prompt(args, named_args),
        "llm.embed" => llm::embed(args, named_args),
        "llm.chat" => llm::chat(args, named_args),

        "notifications.smtp" => notifications::smtp(args, named_args),
        "notifications.slack" => notifications::slack(args, named_args),
        "notifications.telegram" => notifications::telegram(args, named_args),
        "notifications.mattermost" => notifications::mattermost(args, named_args),
        "notifications.gitter" => notifications::gitter(args, named_args),
        "notifications.messenger" => notifications::messenger(args, named_args),
        "notifications.discord" => notifications::discord(args, named_args),
        "notifications.teams" => notifications::teams(args, named_args),
        "notifications.x" => notifications::x(args, named_args),
        "notifications.os" => notifications::os_notify(args, named_args),
        "notifications.irc" => notifications::irc(args, named_args),

        // Type methods
        s if s.starts_with("string.") => {
            crate::type_system::type_methods::call_string_method(s, args)
        }
        s if s.starts_with("number.") => {
            crate::type_system::type_methods::call_number_method(s, args)
        }
        s if s.starts_with("list.") => crate::type_system::type_methods::call_list_method(s, args),
        s if s.starts_with("map.") => crate::type_system::type_methods::call_map_method(s, args),
        s if s.starts_with("re.") => crate::type_system::type_methods::call_re_method(s, args),

        _ => Err(CorvoError::unknown_function(name)),
    }
}
