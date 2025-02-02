# Project Code Combiner

Project Code Combiner is a command-line tool that combines source code files in a project directory into a single file or copies the combined code to the clipboard. This tool outputs the combined code in XML format, which is useful for AI models that can parse structured data more effectively.

## Output Format

The tool combines source code files into a single XML-formatted output. The output format looks like this:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<project>
  <file name="src/main.rs">
    // File contents with proper indentation
    fn main() {
        println!("Hello, World!");
    }
  </file>
  <file name="src/lib.rs">
    // Another file's contents
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
  </file>
</project>
```

This XML format makes it easier for AI models to understand the structure of your project and the relationships between files.

## Installation

To install the Project Code Combiner, follow these steps:

### For Linux and macOS:

1. **Download the Binary**:

   Download the binary for your operating system from the [releases page](https://github.com/kosaki08/project-code-combiner/releases).

2. **Extract the Binary**:

   Extract the downloaded archive to a directory of your choice, for example, `~/pcc`.

3. **Copy the Binary**:

   Copy the `pcc` binary to a directory included in your system's PATH, such as `/usr/local/bin`:

   ```bash
   sudo cp ~/pcc/pcc /usr/local/bin/
   ```

4. **Set Execute Permissions**:

   Ensure that the binary is executable:

   ```bash
   sudo chmod +x /usr/local/bin/pcc
   ```

5. **Copy the Configuration File**:

   Copy the example configuration file to your home directory:

   ```bash
   cp ~/pcc/pcc_config.example.toml ~/.pcc_config.toml
   ```

6. **Customize the Configuration File**:

   Open the configuration file `~/.pcc_config.toml` in a text editor and customize the settings according to your preferences.

7. **Verify Installation**:

   Test the installation by running the command from any location:

   ```bash
   pcc --version
   ```

### For Windows:

1. **Download the Binary**:

   Download the binary for Windows from the [releases page](https://github.com/kosaki08/project-code-combiner/releases).

2. **Extract the Binary**:

   Extract the downloaded archive to a directory of your choice, for example, `C:\pcc`.

3. **Add to System PATH**:

   Add the directory containing the `pcc.exe` binary to your system's PATH environment variable. This allows you to run the command from any location in the command prompt.

4. **Copy the Configuration File**:

   Copy the example configuration file to your user profile directory:

   ```bash
   copy C:\pcc\pcc_config.example.toml %USERPROFILE%\.pcc_config.toml
   ```

5. **Customize the Configuration File**:

   Open the configuration file `%USERPROFILE%\.pcc_config.toml` in a text editor and customize the settings according to your preferences.

6. **Verify Installation**:

   Test the installation by running the command from any location in the command prompt:

   ```bash
   pcc --version
   ```

## Usage

To combine files, use the following command in your terminal:

```bash
$ pcc [OPTIONS] <PROJECT_DIRECTORY>
```

You can include options to override the default behavior specified in the configuration file `.pcc_config.toml`.

### Options

Here are the available command-line options for customizing the execution:

| Option                      | Description                                                                                       |
| --------------------------- | ------------------------------------------------------------------------------------------------- |
| `--copy`                    | Copies the combined source code to the clipboard instead of saving it to a file.                  |
| `--save`                    | Saves the combined source code to a file. File output destinations can override default settings. |
| `--output_path=<PATH>`      | Specifies the output file path for the combined source code.                                      |
| `--ignore_file_path=<PATH>` | Specifies the ignore file path in .gitignore format.                                              |
| `--ignore=<PATTERN>`        | Adds an additional ignore pattern (can be used multiple times).                                   |
| `--help`                    | Displays the help message.                                                                        |
| `--version`                 | Displays the version information.                                                                 |
| `--relative`                | Uses relative paths for file references (default: true).                                          |
| `--no-relative`             | Uses absolute paths for file references.                                                          |

## Examples

### Basic Usage:

```bash
$ pcc </path/to/project> [OPTIONS]
```

This command processes the files in the specified project directory and performs the default actions (copy to clipboard or save to file) listed in the configuration file `.pcc_config.toml`. Override the default action if the following options are given.

### Using Clipboard:

```bash
$ pcc </path/to/project> --copy
```

This command processes the files and copies the combined source code directly to the clipboard, without saving it to a file.

### Using Save to File:

```bash
$ pcc </path/to/project> --save
```

This command processes the files and saves the combined source code to the default output file path specified in the configuration file.

### Using Custom Output Path:

```bash
$ pcc </path/to/project> --output_path=/path/to/output/file
```

This command processes the files and saves the combined source code to the specified output file path.

### Using Custom Ignore File:

```bash
$ pcc </path/to/project> --ignore_file_path=/path/to/custom/ignore.file
```

This allows you to use a custom ignore file instead of the default ignore patterns specified in the configuration file.

### Using Additional Ignore Patterns:

```bash
$ pcc </path/to/project> --ignore=*.log --ignore=temp/ --ignore=*.bak
```

This command processes the files, ignoring files that match the patterns `*.log`, `temp/`, and `*.bak`, in addition to the ignore patterns specified in the configuration file.

### Using Relative Paths:

```bash
$ pcc </path/to/project> --relative
```

This command processes the files and uses relative paths for file references in the combined source code. This is the default behavior.

### Using Absolute Paths:

```bash
$ pcc </path/to/project> --no-relative
```

This command processes the files and uses absolute paths for file references in the combined source code.

## Building from Source

If you prefer to build the binary from the source code, follow these steps:

1. Clone the repository:

   ```bash
   git clone https://github.com/kosaki08/project-code-combiner.git
   cd project-code-combiner
   ```

2. Build the binary:

   ```bash
   cargo build --release
   ```

   The binary will be generated in the `target/release` directory.

3. Run the tool using `cargo run`:

   ```bash
   cargo run -- [OPTIONS] <PROJECT_DIRECTORY>
   ```

   Note the extra `--` after `cargo run` to pass the options and arguments to the tool.

## Configuration

The configuration file `.pcc_config.toml` should be placed in the user's home directory. It allows you to specify default settings for the tool.

Example configuration file:

```toml
[default]
action = "copy"
output_path = "~/combined_code"
output_file_name = "combined_code.txt"
ignore_patterns = [
    "target",
    "*.log",
    "*.txt",
]
use_relative_paths = true
```

## Format of the Ignore File

The ignore file can be written in .gitignore format. You can specify the ignore file path using the `--ignore_file_path` option.

## Uninstallation

### For Linux and macOS:

To uninstall the tool, remove the binary from the directory where it was copied and delete the configuration file:

```bash
sudo rm /usr/local/bin/pcc
rm ~/.pcc_config.toml
```

### For Windows:

To uninstall the tool, remove the `pcc.exe` binary from the directory where it was copied, remove the directory from the system's PATH environment variable, and delete the configuration file:

```bash
del %USERPROFILE%\.pcc_config.toml
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
