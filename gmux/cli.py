import os
import re
import sys
from typing import List, Optional
import click
import time
from colorama import init, Fore

from gmux.config import DEFAULT_PR_TEMPLATE, DEFAULT_PR_TEMPLATE_NAME
from gmux.helper import (
    clone_repository,
    get_base_branch_name,
    get_diff_file_names,
    get_repository_metadata,
    get_status,
    is_git_directory,
)
from gmux.helper import create_pr, get_repositories, get_template
from gmux.utils import _for_each_repository, run_command


@click.group()
def gmux():
    pass


@gmux.command()
@click.argument("directory_arg", required=False)
@click.option("--directory", required=False)
def init(directory_arg: Optional[str], directory: Optional[str]):
    """
    Initialize a new directory for gmux.

    Args:
        directory (str): Directory name provided as an argument.
    """
    dir = directory_arg or directory or "gmux"

    os.makedirs(dir, exist_ok=True)

    with open(f"{dir}/{DEFAULT_PR_TEMPLATE_NAME}", "w") as f:
        f.write(DEFAULT_PR_TEMPLATE)

    click.echo(Fore.GREEN + "✨ gmux successfully initialised! ✨" + Fore.RESET)
    click.echo(f"Change your directory to `{dir}` to begin...")


@gmux.command()
@click.argument("cmd", nargs=-1, type=click.UNPROCESSED)
@click.option("--filter", required=False)
def cmd(cmd: List[str], filter: Optional[str]):
    """
    Run a command in each repository.

    Args:
        cmd (str): Command to run in each repository.
        filter (str): Regex filter for repository names.
    """

    def _cmd(folder):
        click.echo(Fore.WHITE + folder + Fore.RESET + " " + " ".join(cmd))
        return run_command(cmd, cwd=folder, log_metadata=True)

    results = _for_each_repository(_cmd, filter)

    if set([result.returncode for result in results]) != set([0]):
        sys.exit(1)


@gmux.command()
@click.option("--filter", required=False)
def status(filter: Optional[str]):
    """
    Retrieve status for every repository.

    Args:
        filter (str): Regex filter for repository names.
    """

    def _status(folder):
        repository_metadata = get_repository_metadata(folder)

        if not repository_metadata:
            return

        click.echo(
            Fore.WHITE
            + folder
            + Fore.RESET
            + f" {repository_metadata.current_branch} ({repository_metadata.head_commit_ref[0:6]})"
        )

        get_status(folder)

    _for_each_repository(_status, filter=filter)


@gmux.command()
@click.option("--title", prompt=True)
@click.option("--filter", required=False)
def pr(title: str, filter: Optional[str]):
    """
    Create a pull request for each repository.

    Args:
        title (str): Title for the Pull Request.
        filter (str): Regex filter for repository names.
    """
    template = get_template()

    if not template:
        click.echo("PR template not found. Run 'gmux init' first.")
        return

    def _pr(folder):
        base_branch = get_base_branch_name(folder)

        diff_files = get_diff_file_names(folder, base_branch)

        if not diff_files:
            return

        pr_content = template.render(
            title=title, diff_files=diff_files, repository_name=folder
        )

        create_pr(folder, title, pr_content)

    _for_each_repository(_pr, filter)


@gmux.command(
    context_settings=dict(
        ignore_unknown_options=True,
    )
)
@click.argument("git_command", nargs=-1)
@click.option("--filter", required=False)
def git(git_command: List[str], filter: Optional[str]):
    """
    Run any Git command (with magic variables) for all repositories.

    Args:
        filter (str): Regex filter for repository names.
    """

    def _git(folder):
        repository_metadata = get_repository_metadata(folder)

        if not repository_metadata:
            return

        magic_variables = {
            "@default": repository_metadata.default_branch,
            "@current": repository_metadata.current_branch,
        }

        cmd = []

        for variable, value in magic_variables.items():
            cmd = [arg.replace(variable, value) for arg in git_command]

        start_time = time.time()

        result = run_command(["git", *cmd], cwd=folder, text=True, capture_output=True)

        elapsed_time = time.time() - start_time

        return_code_color = Fore.RED if result.returncode != 0 else Fore.WHITE

        click.echo(
            Fore.WHITE
            + folder
            + Fore.RESET
            + f" ({repository_metadata.current_branch}) git {' '.join(cmd)}\n"
            f"{result.stdout}"
            f"{return_code_color} {result.returncode} (elapsed time: {elapsed_time:.2f} seconds){Fore.RESET}"
        )

    _for_each_repository(_git, filter=filter, parallel=True)


@gmux.command()
@click.option("--org", required=True)
@click.option("--filter", required=False)
def clone(org: str, filter: Optional[str]):
    """
    Clone repositories from a specified organization or user.

    Args:
        org (str): Organization or user name.
        filter (str): Regex filter for repository names.
    """
    repositories = get_repositories(org)

    processes = []

    for repository in repositories:
        if filter and not re.match(filter, repository["name"]):
            continue

        if is_git_directory(repository["name"]):
            click.echo(f"Skipping {org}/{repository['name']}, already exists")
            continue

        click.echo(f"cloning {org}/{repository['name']}")
        process = clone_repository(org, repository["name"])
        processes.append(process)

    successful = []
    failed = []

    for process in processes:
        if process.wait() == 0:
            successful.append(process)
        else:
            failed.append(process)

    click.echo(Fore.GREEN + f"Cloned {len(successful)} repositories" + Fore.RESET)

    if failed:
        click.echo(Fore.RED + f"{len(processes)} failed" + Fore.RESET)


if __name__ == "__main__":
    init()
    gmux()
