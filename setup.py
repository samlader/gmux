from setuptools import setup, find_packages

setup(
    name="gmux",
    author="Sam Lader",
    version="0.1",
    packages=find_packages(),
    install_requires=["Jinja2==3.1.3", "click==8.1.7", "ollama==0.1.8"],
    entry_points="""
        [console_scripts]
        gmux=gmux.cli:gmux
    """,
)
