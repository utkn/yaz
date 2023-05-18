use cursive_frontend::CursiveFrontend;
use document::{
    primitive_mods::{DocMapMod, PrimitiveMod},
    Document, DocumentMap, Transaction,
};
use editor::{editor_mode::*, editor_server::EditorServer, HistoricalEditorState, ModalEditor};

use highlight_server::HighlightServer;
use render_server::RendererServer;

mod cursive_frontend;
mod cursor;
mod document;
mod editor;
mod events;
mod highlight_server;
mod render_server;

fn main() {
    let file_name = std::env::args().nth(1).unwrap_or_default();
    // Initialize the editor state with the file.
    let mut editor_state: HistoricalEditorState = DocumentMap::default().into();
    editor_state.modify_with_tx(
        &Transaction::new()
            .with_mod(PrimitiveMod::DocMap(DocMapMod::PopDoc(0)))
            .with_mod(PrimitiveMod::DocMap(DocMapMod::CreateDoc(
                Document::new_from_file(&file_name),
            ))),
    );
    // Construct the editor.
    let editor = ModalEditor::new(editor_state, NormalMode::id())
        .with_mode(Box::new(InsertMode::new()))
        .with_mode(Box::new(NormalMode::new()))
        .with_mode(Box::new(GotoMode::new()))
        .with_mode(Box::new(CommandMode::new()))
        .with_mode(Box::new(SelectionMode::new()));
    // Construct the servers.
    let mut editor_server = EditorServer::new(editor);
    let mut rnd_server = RendererServer::<CursiveFrontend>::new(editor_server.new_connection());
    let mut hl_server = HighlightServer::new(editor_server.new_connection());
    let mut cursive_ctx = rnd_server.get_frontend_mut().init_cursive_context();
    // Run in the background.
    hl_server.run();
    rnd_server.run();
    editor_server.run();
    // Run in the main thread.
    cursive_ctx.run();
}
