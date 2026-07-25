import os


def _get_project_root():
    current_file_path = os.path.abspath(__file__)
    helpers_dir = os.path.dirname(current_file_path)
    root_dir = os.path.dirname(helpers_dir)
    return root_dir


PROJECT_ROOT_DIR = _get_project_root()
CONFIG_DIR = os.path.join(PROJECT_ROOT_DIR, "config")
TMP_DIR = os.path.join(PROJECT_ROOT_DIR, "tmp")
PROPERTY_INDEX_TXT_PATH = os.path.join(CONFIG_DIR, "property_index.txt")
