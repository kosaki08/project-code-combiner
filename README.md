## Usage

To combine files, use the following command in your terminal:

```
$ cargo run <project_directory>
```

You can also include options to customize the behavior of the application as described below.

### Optinos

Here are the available command line options for customizing the execution:

| Option                                     | Description                                                                                                                                |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `--clipboard`                              | Copies the combined source code to the clipboard instead of saving it to a file.                                                           |
| `--output_path`                            | Specify the output directory of the combined source code.                                                                                  |
| `--ignore_file_path=<path/to/ignore/file>` | Specifies a custom path to an ignore file, which is used to exclude files from being combined. Defaults to `.pcc_ignore` if not specified. |

## Examples

### Basic Usage:

```
$ cargo run /path/to/project
```

This command processes the files in the specified project directory, saving the combined source code to a file in the project directory.

### Using Clipboard:

```
$ cargo run /path/to/project
```

This command processes the files and copies the combined source code directly to the clipboard, without saving it to a file.

### Using Custom Output Path:

```
$ cargo run /path/to/project --output_path=/path/to/output/file
```

This command processes the files and saves the combined source code to the specified output file path.

### Using Custom Ignore File:

```
$ cargo run /path/to/project --ignore_file_path=/path/to/custom/ignore.file
```

This allows you to use a custom ignore file instead of .pcc_ignore.

## Format of the Ignore File

Configuration file can be written in .gitignore format. Place the configuration file `.pcc_ignore` in the project root or specify a custom path using the `--ignore_file_path` option.
