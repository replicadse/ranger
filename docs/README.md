# ranger

`ranger` is tool for templating entire folder structures using the comprehensive `handlebars` syntax.

Examples:

* `ranger generate local -f ./templates/example -o ./test --force`
    * `ranger generate local`: Generate via a local template.
    * `-f ./templates/example`: The location of the template to use.
    * `-o ./test`: The output folder.
    * `--force`: Force overwriting the folder if it exists (delete & recreate).
* `ranger generate git --repo "https://github.com/replicadse/ranger" --branch master --folder ./templates/example -o ./test`
    * `ranger generate git`: Generate via git repo (will temporarily check out to a temp dir that is cleared after use).
    * `--repo "https://github.com/replicadse/ranger"`: The repository containing the template.
    * `--branch`: The branch to check out.
    * `--folder`: The folder to use within the repository.
    * `-o ./test`: The output folder.

# Rangerfile

If the template folder (local, git, ...) contains a `.ranger.yaml` file, further information might be specified in there. This includes variable default values, helper functions etc.
