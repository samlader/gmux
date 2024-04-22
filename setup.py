from setuptools import setup, find_packages

setup(
    name="gmux",
    author="Sam Lader",
    description="CLI tool to manage & automate repetitive Git workflows across multiple Github repositories.",
    version="0.2",
    packages=find_packages(),
    install_requires=["Jinja2==3.1.3", "click==8.1.7", "colorama==0.4.6", "ollama==0.1.8"],
    entry_points="""
        [console_scripts]
        gmux=gmux.cli:gmux
    """,
)
