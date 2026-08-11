pub mod browse;
pub mod details;
pub mod downloads;
pub mod extensions;
pub mod home;
pub mod library;
pub mod reader;
pub mod settings;
pub mod shell;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Home,
    Browse,
    Details,
    Library,
    Reader,
    Downloads,
    Settings,
    Extensions,
}