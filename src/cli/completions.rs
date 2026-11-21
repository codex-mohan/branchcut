use crate::error::{AppError, Result};

const FLAGS: &str = "--glob --exclude --extension --type --empty --executable --owner --and --content --grep --pattern-file --regex --content-mode --word-regexp --line-regexp --invert-match --text --encoding --max-line-bytes --paths-from --paths-from0 --preprocess --decompress --after-context --before-context --context --only-matching --files-with-matches --files-without-match --count-per-file --output --explain-matches --summary --ignore-case --case-sensitive --smart-case --unicode --escaped-globs --full-path --filename --size --modified-within --modified-before --min-depth --max-depth --cwd --search-path --hidden --follow --one-file-system --max-open --contents-first --first --limit --sort --count --quiet --has-results --json --json-metadata --print0 --absolute --format --color --hyperlink --exec --exec-batch --batch-size --exec-jobs --exec-preview --stats --explain --why --config --profile --gitignore --ignore --ignore-file --threads --strict --completions --help --version";

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
    if [[ $COMP_CWORD -eq 1 ]]; then
      COMPREPLY+=( $(compgen -W 'find grep' -- "$cur") )
    fi
    while IFS= read -r candidate; do
      COMPREPLY+=("$candidate")
    done < <(compgen -f -- "$cur")
  fi
}}
complete -F _branchcut_complete branchcut branchcut-regex"#
        ),
        "zsh" => {
            println!(
                "#compdef branchcut branchcut-regex\n_branchcut() {{\n  local -a options\n  if (( CURRENT == 2 )); then\n    _values 'search command' 'find[Search filenames]' 'grep[Search file contents]'\n  fi\n  options=("
            );
            for flag in FLAGS.split_whitespace() {
                println!("    '{flag}'");
            }
            println!(
                "  )\n  _describe 'branchcut option' options\n  _files\n}}\ncompdef _branchcut branchcut branchcut-regex"
            );
        }
        "fish" => {
            println!("# Branchcut completions; save as branchcut.fish");
            for command in ["branchcut", "branchcut-regex"] {
                println!("complete -c {command} -n '__fish_use_subcommand' -a 'find grep'");
            }
            for flag in FLAGS.split_whitespace() {
                println!("complete -c branchcut -c branchcut-regex -l {}", &flag[2..]);
            }
        }
        "powershell" | "pwsh" => {
            println!(
                "Register-ArgumentCompleter -Native -CommandName branchcut,branchcut-regex -ScriptBlock {{\n  param($wordToComplete, $commandAst, $cursorPosition)\n  @(\n    if ($commandAst.CommandElements.Count -le 2 -and $wordToComplete -notlike '-*') {{ 'find'; 'grep' }}"
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
