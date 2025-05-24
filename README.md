# gmux

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/samlader/gmux/releases/tag/v1.0.0)
[![CI](https://github.com/samlader/gmux/actions/workflows/ci.yml/badge.svg)](https://github.com/samlader/gmux/actions/workflows/ci.yml)

A simple command-line tool designed to automate repetitive Git workflows across multiple Github repositories.

Common tasks like cloning repositories and performing commits occur in parallel, while pull requests are dynamically generated - enabling you to ship changes at **lightning speed**. ⚡

## Installation

Install **gmux** using one of the following methods:

### Using Homebrew (Recommended)

```bash
brew tap samlader/tap
brew install gmux
```

### Using Cargo (from source)

```bash
# Install Rust (optional)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

cargo install --git https://github.com/samlader/gmux.git --tag latest
```

## Usage

### 1. Setup

Setup your GitHub authentication by running the setup command. This will guide you through creating a GitHub personal access token with the required permissions:

```bash
gmux setup
```

Use the init command to create a new working directory for gmux, along with a pull request template:

```bash
gmux init --directory=<directory_name>
```

### 2. List Repositories

List all repositories for a specified GitHub organization or user:

```bash
gmux list <organization_or_user>
```

### 3. Clone Multiple Repositories

Clone all repositories from a specified GitHub organization or user:

```bash
gmux clone <organization_or_user> [--filter=<regex_filter>]
```

### 4. Git Commands

Execute any Git command for all repositories. Dynamic variables for each repository can be used.

```bash
gmux git [GIT_COMMAND] [--filter=<regex_filter>]
```

#### Dynamic Variables

- `@default` (default branch of a repository)
- `@current` (current branch of the repository)

### 5. Dynamic Pull Requests

Create pull requests for each repository:

```bash
gmux pr --title "My PR Title"
```

<!--
> [!NOTE]
> This command will launch pre-populated draft in your browser. For safety reasons, submission of a PR is a manual action. -->

Pull requests use the template (`PR_TEMPLATE.md`) created in the root directory by default.

#### Features

##### Jinja Expressions

Templates support [Jinja](https://jinja.palletsprojects.com/en/3.1.x/) expressions and the following context variables are provided:

- `repository_name` (name of the repository)
- `diff_files` (files with changes against the base branch)

#### Example template

```jinja
## Overview

This PR contains {{ diff_files|length }} changes for {{ repository_name }}.

{% if "README.md" in diff_files %}
The documentation has been updated to reflect these changes accordingly.
{% endif %}

## Changes

{% for diff_file in diff_files %}
- {{ diff_file }}
{% endfor %}
```

### 6. Arbitrary Commands

Execute a command in each repository. Useful for batch operations across multiple projects.

```bash
gmux cmd [COMMAND] [--filter=<regex_filter>]
```

## Examples

```bash
# Initialize a new directory
gmux init

# Clone service repositories from the organization "example-org"
gmux clone --org=example-org --filter="*-service"

# Create a new branch on all repositories
gmux git checkout -b feature-branch

# Run codemod across all repositories
codemod -m --extensions html \
    '<font *color="?(.*?)"?>(.*?)</font>' \
    '<span style="color: \1;">\2</span>'

# Commit changes
gmux git commit -m "Implement new feature"

# Create pull requests for all repositories
gmux pr --title "Implement new feature"
```

## Makefile

A `Makefile` is provided for convenience. The main targets are:

- `make build` - Build the project
- `make test` - Run all tests (unit and integration)
- `make fmt` - Format the codebase
- `make check` - Check code without building
- `make clean` - Remove build artifacts

## Contributions

Contributions and bug reports are welcome! Feel free to open issues, submit pull requests or contact me if you need any support.

## License

This project is licensed under the [MIT License](LICENSE).
