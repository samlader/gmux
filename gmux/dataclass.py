from dataclasses import dataclass


@dataclass
class RepositoryMetadata:
    default_branch: str
    current_branch: str
    head_commit_ref: str
