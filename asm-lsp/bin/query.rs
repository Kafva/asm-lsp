use lsp_types::WorkspaceFolder;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use asm_lsp::types::Config;
use asm_lsp::{Arch, Assembler, ServerStore, get_root_config};
use asm_lsp::{DocumentStore, RootConfig, get_hover_resp};
use clap::Args;
use lsp_types::{
    HoverContents, HoverParams, InitializeParams, MarkupContent, MarkupKind, Position,
    TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
};

#[derive(Args, Debug, Clone)]
#[command(about = "Get information about token")]
pub struct QueryGetArgs {
    #[arg(long, short, help = "Name of directive, register or opcode")]
    pub name: Option<String>,
}

pub fn query_get(name: &str) -> Result<()> {
    let (uri, _, config, store) = setup()?;

    // Placeholder values, based of hovering unit test code
    let pos_params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position::default(),
    };
    let hover_params = HoverParams {
        text_document_position_params: pos_params.clone(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let cursor_offset = 0;
    let mut doc_store = DocumentStore::new();

    let Some(resp) = get_hover_resp(
        &hover_params,
        &config,
        name,
        cursor_offset,
        &mut doc_store,
        &store,
    ) else {
        return Err(anyhow!(
            "Received empty hover response for name: '{}'",
            name
        ));
    };

    if let HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: resp_text,
    }) = resp.contents
    {
        let cleaned = resp_text
            .replace("\n\n\n\n", "\n\n") // HACK:: not sure what's going on here...
            .replace("\n\n\n", "\n\n"); // ...or here
        println!("{}", cleaned);
        return Ok(());
    } else {
        return Err(anyhow!(
            "Invalid hover response contents: {:?}",
            resp.contents
        ));
    }
}

pub fn query_list() -> Result<()> {
    let (_, _, _, store) = setup()?;
    let maps = store.names_to_info;

    let ins_names: Vec<String> = maps
        .instructions
        .into_iter()
        .map(|info| info.1.name.clone())
        .collect();
    let reg_names: Vec<String> = maps
        .registers
        .into_iter()
        .map(|info| info.1.name.clone())
        .collect();
    let dir_names: Vec<String> = maps
        .directives
        .into_iter()
        .map(|info| info.1.name.clone())
        .collect();

    let ins = ins_names.join("\n");
    let reg = reg_names.join("\n");
    let dir = dir_names.join("\n");

    println!("{}\n{}\n{}", reg, dir, ins);
    Ok(())
}

fn setup() -> Result<(Uri, RootConfig, Config, ServerStore)> {
    if let Ok(level) = std::env::var("RUST_LOG") {
        flexi_logger::Logger::try_with_str(level)?.start()?;
    }

    let cwd = std::env::current_dir()?;
    let cwd_name = String::from_str(cwd.file_name().unwrap().to_str().unwrap())?;
    let uri: Uri = Uri::from_str(cwd.to_str().expect("bad working directory"))?;

    let workspace_folder = WorkspaceFolder {
        uri: uri.clone(),
        name: cwd_name,
    };
    let params: InitializeParams = InitializeParams {
        workspace_folders: Some(vec![workspace_folder]),
        ..Default::default()
    };

    let root_config = get_root_config(&params)?;
    let config = root_config.get_config(&uri);
    let store = get_server_store(&root_config);

    Ok((uri.clone(), root_config.clone(), config.clone(), store))
}

fn get_server_store(config: &RootConfig) -> ServerStore {
    let mut store = ServerStore::default();

    // Populate names to `Instruction`/`Register`/`Directive` maps
    for isa in config.effective_arches() {
        isa.setup_instructions(None, &mut store.names_to_info.instructions);
        isa.setup_registers(&mut store.names_to_info.registers);
    }

    for assembler in config.effective_assemblers() {
        assembler.setup_directives(&mut store.names_to_info.directives);
        if assembler == Assembler::Mars {
            Arch::Mips
                .setup_instructions(Some(Assembler::Mars), &mut store.names_to_info.instructions);
        }
    }

    return store;
}
