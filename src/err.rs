#[derive(Debug)]
pub enum Error {
    DlOpenError,
    DlSymError,
    FindBaseAddrError,
    PlayersListError,
    TraceLineError,
    Player1Error,
    SDLHookError,
    SymbolError,
}
