CREATE INDEX IF NOT EXISTS messages_from_history ON messages(from_name,from_host,from_incarnation,id);
CREATE INDEX IF NOT EXISTS messages_to_history ON messages(to_name,to_host,to_incarnation,id)
    WHERE (from_name,from_host,from_incarnation)<>(to_name,to_host,to_incarnation);
