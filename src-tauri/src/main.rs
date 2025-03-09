// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod sqlite;

fn main() {
    sqlite::init_db();
    boombox_lib::run()
}
