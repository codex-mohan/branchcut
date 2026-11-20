use crate::error::{AppError, Result};

const FLAGS: &str = "--glob --exclude --extension --type --empty --executable --owner --and --content --grep --text --files-with-matches --ignore-case --case-sensitive --smart-case --unicode --escaped-globs --full-path --filename --size --modified-within --modified-before --min-depth --max-depth --cwd --search-path --hidden --follow --one-file-system --max-open --contents-first --first --limit --sort --count --quiet --has-results --json --json-metadata --print0 --absolute --format --color --hyperlink --exec --exec-batch --batch-size --exec-jobs --exec-preview --stats --explain --why --config --profile --gitignore --ignore --ignore-file --threads --strict --completions --help --version";

/// Emit sourceable completion definitions without reading project files.
pub(crate) fn print(shell: &str) -> Result<()> {
    match shell {
        "bash" => println!(
            r#"_branchcut_complete() {{
  local cur="${{COMP_WORDS[COMP_CWORD]}}"
  if [[ "$cur" == -* ]]; then
    COMPREPLY=( $(compgen -W '{FLAGS}' -- "$cur") )
  else
    COMPREPLY=()
    while IFS= read -r candidate; do
      COMPREPLY+=("$candidate")
    done < <(compgen -f -- "$cur")
  fi
}}
complete -F _branchcut_complete branchcut"#
        ),
        "zsh" => {
            println!("#compdef branchcut\n_branchcut() {{\n  local -a options\n  options=(");
            for flag in FLAGS.split_whitespace() {
                println!("    '{flag}'");
            }
            println!(
                "  )\n  _describe 'branchcut option' options\n  _files\n}}\ncompdef _branchcut branchcut"
            );
        }
        "fish" => {
            println!("# Branchcut completions; save as branchcut.fish");
            for flag in FLAGS.split_whitespace() {
                println!("complete -c branchcut -l {}", &flag[2..]);
            }
        }
        "powershell" | "pwsh" => {
            println!(
                "Register-ArgumentCompleter -Native -CommandName branchcut -ScriptBlock {{\n  param($wordToComplete, $commandAst, $cursorPosition)\n  @("
            );
            for flag in FLAGS.split_whitespace() {
                println!("    '{flag}'");
            }
            println!(
                "  ) | Where-Object {{ $_ -like \"$wordToComplete*\" }} | ForEach-Object {{\n    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterName', $_)\n  }}\n}}"
            );
        }
        _ => {
            return Err(AppError(
                "--completions expects bash, zsh, fish, or powershell".to_owned(),
            ));
        }
    }
    Ok(())
}
