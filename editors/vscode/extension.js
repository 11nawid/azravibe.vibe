// Azra (.vibe) VS Code extension — activates azra-lsp and adds a run command.
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
  const configured = vscode.workspace
    .getConfiguration('azra')
    .get('languageServer', 'azra-lsp');

  client = new LanguageClient(
    'azra',
    'Azra Language Server',
    { command: configured, args: [] },
    {
      documentSelector: [{ scheme: 'file', language: 'vibe' }],
      synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher('**/*.vibe') },
    }
  );
  context.subscriptions.push(client.start());

  context.subscriptions.push(
    vscode.commands.registerCommand('azra.runFile', () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'vibe') {
        vscode.window.showInformationMessage('Open a .vibe file first.');
        return;
      }
      const terminal =
        vscode.window.activeTerminal || vscode.window.createTerminal('Azra');
      terminal.show();
      terminal.sendText(`azra run "${editor.document.fileName}"`);
    })
  );
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
