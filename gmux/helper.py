import os
import shlex
import subprocess

import json
from typing import List, Optional

from jinja2 import Template

from gmux.config import DEFAULT_PR_TEMPLATE_NAME
from gmux.dataclass import RepositoryMetadata
from gmux.macros import ollama_chat


def get_template(template_path=None) -> Template:
    if not template_path:
        template_path = DEFAULT_PR_TEMPLATE_NAME

    if not os.path.isfile(template_path):
        return

    with open(template_path, "r") as f:
        template_content = f.read()

    template = Template(template_content)

    template.globals.update(ollama_chat=ollama_chat)

    return template


def clone_repository(org, repository, shallow=True) -> subprocess.Popen:
    process = subprocess.Popen(
        f"gh repo clone {org}/{repository} -- --depth=1", shell=True
    )
    return process


def get_base_branch_name(folder) -> str:
    return subprocess.check_output(
        "git remote show origin | sed -n '/HEAD branch/s/.*: //p'",
        shell=True,
        cwd=folder,
        text=True,
    ).strip()


def get_current_branch_name(folder) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=folder, text=True
    ).strip()


def get_head_commit_ref(folder) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=folder, text=True
    ).strip()


def get_diff_file_names(folder, base_branch) -> List[str]:
    return (
        subprocess.check_output(
            f"git diff --name-only origin/{base_branch}",
            shell=True,
            cwd=folder,
            text=True,
        )
        .strip()
        .split("\n")
    )


def get_repository_metadata(folder) -> Optional[RepositoryMetadata]:
    if not is_git_directory(folder):
        return

    return RepositoryMetadata(
        default_branch=get_base_branch_name(folder),
        current_branch=get_current_branch_name(folder),
        head_commit_ref=get_head_commit_ref(folder),
    )


def get_status(folder) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "status", "-s"], cwd=folder)


def is_git_directory(folder) -> bool:
    return os.path.isdir(f"{folder}/.git")


def create_pr(folder, title, pr_content) -> subprocess.CompletedProcess:
    subprocess.run(
        f'gh pr create -w --title {shlex.quote(title)} --body {shlex.quote(pr_content)}',
        shell=True,
        cwd=folder,
    )


def get_repositories(org) -> dict:
    return json.loads(os.popen(f"gh repo list {org} --json name  --limit 9999").read())
