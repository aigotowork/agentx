//! SQLite resolves the preserved --sql predicate under a domain read boundary.
//! Authorizer checks apply to subqueries and views, not only the outer statement.
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

#[derive(Clone, Copy)]
pub(crate) enum Projection {
    Events,
    Participants,
}

pub(crate) fn read<T>(connection: &Connection, projection: Projection, action: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            let allowed = match context.action {
                AuthAction::Select | AuthAction::Recursive => true,
                AuthAction::Read { table_name, .. } => {
                    let table = table_name.to_ascii_lowercase();
                    let owned = match projection {
                        Projection::Events => matches!(table.as_str(), "events" | "events_v"),
                        Projection::Participants => table == "instances",
                    };
                    (owned || matches!(table.as_str(), "json_each" | "json_tree")) && context.database_name.is_none_or(|name| name == "main")
                }
                AuthAction::Function { function_name } => {
                    let name = function_name.to_ascii_lowercase();
                    name.starts_with("json_")
                        || matches!(
                            name.as_str(),
                            "json"
                                | "jsonb"
                                | "abs"
                                | "avg"
                                | "char"
                                | "coalesce"
                                | "concat"
                                | "concat_ws"
                                | "count"
                                | "date"
                                | "datetime"
                                | "glob"
                                | "group_concat"
                                | "hex"
                                | "ifnull"
                                | "iif"
                                | "instr"
                                | "julianday"
                                | "length"
                                | "like"
                                | "likelihood"
                                | "likely"
                                | "lower"
                                | "ltrim"
                                | "max"
                                | "min"
                                | "nullif"
                                | "printf"
                                | "quote"
                                | "replace"
                                | "round"
                                | "rtrim"
                                | "sign"
                                | "strftime"
                                | "substr"
                                | "substring"
                                | "sum"
                                | "time"
                                | "total"
                                | "trim"
                                | "typeof"
                                | "unicode"
                                | "unixepoch"
                                | "unlikely"
                                | "upper"
                        )
                }
                _ => false,
            };
            if allowed { Authorization::Allow } else { Authorization::Deny }
        }))
        .map_err(|error| error.to_string())?;
    struct Reset<'a>(&'a Connection);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            let _ = self.0.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        }
    }
    let _reset = Reset(connection);
    action()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_predicates_allow_owned_subquery_and_deny_other_tables_and_statements() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE events(id INTEGER, data TEXT); CREATE VIEW events_v AS SELECT * FROM events; CREATE TABLE kv(value TEXT); INSERT INTO events VALUES(1, '{}');").unwrap();
        let run = |sql: &str| read(&connection, Projection::Events, || connection.query_row(sql, [], |row| row.get::<_, i64>(0)).map_err(|error| error.to_string()));
        assert_eq!(run("SELECT count(*) FROM events_v WHERE EXISTS(SELECT 1 FROM events_v e WHERE e.id=1)").unwrap(), 1);
        assert!(run("SELECT count(*) FROM events_v WHERE EXISTS(SELECT 1 FROM kv)").is_err());
        assert!(run("SELECT count(*) FROM sqlite_master").is_err());
        assert!(read(&connection, Projection::Events, || connection.execute("DELETE FROM events", []).map_err(|error| error.to_string())).is_err());
        connection.execute("INSERT INTO kv VALUES('after guard')", []).unwrap();
    }
}
