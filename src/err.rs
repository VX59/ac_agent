#[derive(Debug)]
pub enum Error {
    DlOpenError,
    DlSymError,
    FindBaseAddrError,
    PlayersListError,
    EntsListError,
    TraceLineError,
    Player1Error,
    SDLHookError,
    SymbolError,
    BehindCamera,
}
