from setuptools import setup, find_packages

setup(
    name="gmux",
    author="Sam Lader",
    version="0.1",
    packages=find_packages(),
    install_requires=[
        "Jinja2",
        "click",
    ],
    entry_points="""
        [console_scripts]
        gmux=gmux.cli:gmux
    """,
)
