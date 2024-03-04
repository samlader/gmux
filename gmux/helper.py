import os
import subprocess
import re
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


def create_pr(folder, title, pr_content):
    subprocess.run(
        f'gh pr create -w --title "{title}" --body "{pr_content}"',
        shell=True,
        cwd=folder,
    )


def get_repositories(org):
    return json.loads(os.popen(f"gh repo list {org} --json name  --limit 9999").read())

def get_codeowners(file_path):
    owners = set()

    codeowners_path = os.path.join(file_path, '.github/CODEOWNERS')

    if os.path.isfile(codeowners_path):
        with open(codeowners_path, 'r') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                pattern, *owner_strings = line.split()
                for owner_string in owner_strings:
                    if owner_string.startswith('@'):
                        owners.add(owner_string[1:])  # remove '@' symbol
                    elif '@' in owner_string:
                        # Extracting the owner email address
                        email = re.findall(r'[^@|\s]+@[^@]+\.[^@|\s]+', owner_string)
                        if email:
                            owners.add(email[0])
                    elif '/' in owner_string:
                        # Extracting team owner
                        team = owner_string.split('/')[-1]
                        owners.add(team)

    return list(owners) if owners else None
