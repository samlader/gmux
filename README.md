# gmux

[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/samlader/gmux/releases/tag/v0.2.0)

A simple command-line tool designed to automate repetitive Git workflows across multiple Github repositories.

Common tasks like cloning repositories and performing commits occur in parallel, while pull requests are dynamically generated - enabling you to ship changes at **lightning speed**. ⚡

## Installation

Before using **gmux**, make sure you have the required dependencies installed:

```bash
# brew install git
git --version

# brew install gh
gh --version
```

Install **gmux** using the following command:

```bash
pip3 install https://github.com/samlader/gmux/archive/refs/tags/v0.2.0.zip
```

## Usage

### 1. Initialize a New Directory

Use the init command to create a new working directory for gmux, along with a pull request template:

```bash
gmux init --directory=<directory_name>
```

### 2. Clone Multiple Repositories

Clone all repositories from a specified GitHub organization or user:

```bash
gmux clone --org=<organization_or_user> [--filter=<regex_filter>]
```

### 3. Git Commands

Execute any Git command for all repositories. Dynamic variables for each repository can be used.

```bash
gmux git [GIT_COMMAND] [--filter=<regex_filter>]
```

#### Dynamic Variables

- `@default` (default branch of a repository)

- `@current` (current branch of the repository)

### 4. Dynamic Pull Requests

Create pull requests for each repository:

```bash
gmux pr
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

##### Text Generation

Templates support LLM-based text generation using the `ollama_chat` macro.

This macro requires a local installation of [ollama](https://ollama.com/) and accepts two arguments:

- `model` (name of the model, all available models can be found [here](https://ollama.com/library))
- `prompt` (text prompt given to the model)

#### Example template

```jinja
## Overview

This PR contains {{ diff_files|length }} changes for {{ repository_name }}.

{% if "README.md" in diff_files %}
The documentation has been updated to reflect these changes accordingly.
{% endif %}

{{ ollama_chat("llama3", "Write some guidelines on the usage of React hooks") }}

## Changes

{% for diff_file in diff_files %}
- {{ diff_file }}
{% endfor %}
```

### 5. Arbitrary Commands

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
gmux cmd codemod -m --extensions html \
    '<font *color="?(.*?)"?>(.*?)</font>' \
    '<span style="color: \1;">\2</span>'

# Commit changes
gmux git commit -m "Implement new feature"

# Create pull requests for all repositories
gmux pr
```

## Contributions

Contributions and bug reports are welcome! Feel free to open issues, submit pull requests or contact me if you need any support.

## License

This project is licensed under the [MIT License](LICENSE).
