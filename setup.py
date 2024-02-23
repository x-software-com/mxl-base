#!/usr/bin/env python3
"""Script to install tools and build VCPKG third party libraries"""

import argparse
import os
import subprocess

def setup_git():
    """Setup git for development"""
    subprocess.run(['git', 'config', 'pull.rebase', 'true'], check = True)
    subprocess.run(['git', 'config', 'branch.autoSetupRebase', 'always'], check = True)


def install_cocogitto():
    """Install cocogitto for conventional commits and version bumping"""
    print("Install cocogitto...")
    subprocess.run(['cargo', 'install', 'cocogitto'], check = True)


def setup_cocogitto():
    """Setup cocogitto for development use"""
    print("Setup cocogitto...")
    subprocess.run(['cog', 'install-hook', '--overwrite', 'commit-msg'], check = True)


def install_just():
    """Install 'just' as a basic 'make' replacement to configure, build, package and run projects"""
    print("Install just...")
    subprocess.run(['cargo', 'install', 'just'], check = True)


def install_typos():
    """Install typos to check the repository for common typos"""
    print("Install typos...")
    subprocess.run(['cargo', 'install', 'typos-cli'], check = True)


def setup_tools(setup_for_ci):
    """Install and setup all tools"""
    if not setup_for_ci:
        setup_git()
    else:
        # Add the current directory as a safe directory to determine the current version number
        subprocess.run(['git', 'config', '--global', '--add', 'safe.directory', os.getcwd()], check = True)

    install_typos()
    install_just()

    install_cocogitto()
    if not setup_for_ci:
        setup_cocogitto()

def setup():
    """Parse command line and call setup functions"""
    parser = argparse.ArgumentParser(description='Setup mithra environment')
    parser.add_argument('--ci', dest='ci', action=argparse.BooleanOptionalAction, default=False, help='Setup for CI pipeline')
    options = parser.parse_args()

    setup_tools(options.ci)


if __name__ == "__main__":
    setup()
