# Project Code Combiner

Project Code Combiner (PCC) is a command-line tool that combines source code files in a project directory into a single file or copy to clipboard. This tool is useful for combining multiple files into a single file for asking questions to AI models or sharing code snippets.

## Usage

To combine files, use the following command in your terminal:

```bash
$ cargo run <project_directory>
```

You can also include options to override the behavior of the application as described below.
You can include options to override the default behavior specified in the configuration file `.pcc_config.toml`.

### Optinos

Here are the available command line options for customizing the execution:

| Option                                     | Description                                                                                         |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------- |
| `--copy`                                   | Copies the combined source code to the clipboard instead of saving it to a file.                    |
| `--save`                                   | Save the combined source code to the fiile. File output destinations can override default settings. |
| `--output_path`                            | Specify the output directory of the combined source code.                                           |
| `--ignore_file_path=<path/to/ignore/file>` | Specify a custom path to an ignored file written in `.gitignore` file format.                       |
| `--help`                                   | Display the help message.                                                                           |
| `--version`                                | Display the version of the script.                                                                  |

## Examples

### Basic Usage:

```bash
$ cargo run /path/to/project
```

This command processes the files in the specified project directory, perform the default actions (copy to clipboard or save to file) listed in the configuration file `.pcc_config.toml`. Override the default action if the following options are given

### Using Clipboard:

```bash
$ cargo run /path/to/project --copy
```

This command processes the files and copies the combined source code directly to the clipboard, without saving it to a file.

### Using Save to File:

```bash
$ cargo run /path/to/project --save
```

This command processes the files and saves the combined source code to the default output file path specified in the configuration file.

### Using Custom Output Path:

```bash
$ cargo run /path/to/project --output_path=/path/to/output/file
```

This command processes the files and saves the combined source code to the specified output file path.

### Using Custom Ignore File:

```bash
$ cargo run /path/to/project --ignore_file_path=/path/to/custom/ignore.file
```

This allows you to use a custom ignore file instead of .pcc_ignore.

## Format of the Ignore File

Configuration file can be written in .gitignore format. Place the configuration file `.pcc_ignore` in the project root or specify a custom path using the `--ignore_file_path` option.

## Global Installation

You can install the binary globally to make it accessible from any location on your system:

### For Linux and macOS:

1. **Copy files**:

   Copy the binary to a directory included in your system's PATH, such as `/usr/local/bin` and copy the configuration file to your home directory:

   ```bash
   sudo cp ./target/release/pcc /usr/local/bin/
   sudo cp ./pcc_config.example.toml ~/.pcc_config.toml
   ```

2. **Set Execute Permissions (if necessary)**:

   Ensure that the binary is executable:

   ```bash
   sudo chmod +x /usr/local/bin/pcc
   ```

3. **Verify Installation**:

   Test the installation by running the command from any location:

   ```bash
   pcc <options>
   ```

## Uninstall

### For Linux and macOS:

To uninstall the binary, simply remove it from the directory where it was copied:

```bash
sudo rm /usr/local/bin/pcc
sudo rm ~/.pcc_config.toml
```

## Development

### Build the Binary

To build the binary, run the following command:

```bash
cargo build --release
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
