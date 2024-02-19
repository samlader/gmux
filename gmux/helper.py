import os
import re
import subprocess
import concurrent.futures

import json
from typing import Optional

from gmux.dataclass import RepositoryMetadata


def clone_repository(org, repository, shallow=True):
    print(f"cloning {org}/{repository}")
    process = subprocess.Popen(
        f"gh repo clone {org}/{repository} -- --depth=1", shell=True
    )
    return process


def get_base_branch_name(folder):
    return subprocess.check_output(
        "git remote show origin | sed -n '/HEAD branch/s/.*: //p'",
        shell=True,
        cwd=folder,
        text=True,
    ).strip()


def get_current_branch_name(folder):
    return subprocess.check_output(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=folder, text=True
    ).strip()


def get_head_commit_ref(folder):
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=folder, text=True
    ).strip()


def get_diff_file_names(folder, base_branch):
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


def get_status(folder):
    return subprocess.run(["git", "status", "-s"], cwd=folder)


def is_git_directory(folder):
    return os.path.isdir(f"{folder}/.git")


def _for_each_repository(function, filter=None, parallel=False, *args, **kwargs):
    folders = [
        folder
        for folder in os.listdir(".")
        if is_git_directory(folder) and (not filter or re.match(filter, folder))
    ]

    if parallel:
        with concurrent.futures.ThreadPoolExecutor() as executor:
            return executor.map(function, ["moodmap"], *args, **kwargs)

    result = []

    for folder in folders:
        try:
            result.append(function(folder, *args, **kwargs))
        except Exception as e:
            print(f"Error for {folder}:\n \033[93m{e}\033[0m")

    return result


def create_pr(folder, title, pr_content):
    subprocess.run(
        f'gh pr create -w --title "{title}" --body "{pr_content}"',
        shell=True,
        cwd=folder,
    )


def get_repositories(org):
    return json.loads(os.popen(f"gh repo list {org} --json name  --limit 9999").read())
