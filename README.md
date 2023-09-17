# monout

A utility to monitor and direct the output of running services to external logging tools. 

## Project Status

This is currently an experiment. Support for outputting to the console and to AWS Cloudwatch are baked in. If I continue
with this project I will make it more easily extensible to pipe logs and metrics anywhere you like. The configuration 
file format will be considered stable once this project reaches version 1. 


## Compiling

For now binaries are not provided. They will be if I continue with this project. 

1. You will need to install Rust and Cargo (https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. In this project directory run `cargo build -r`. There will likely be warnings up until the v1 release.
3. The `monout` binary will be in `target/release`. 
4. If you need the binary for a different platform please consult the Cargo documentation. 

## Usage

Define a YAML file that contains your runners: 

```yaml
runners:
  Hello:
    command: "echo hello && sleep 2"
    output_type: aws
    aws:
      log_group: test-thing
      log_stream_prefix: hello
      region: us-west-2

  Goodbye:
    command: "echo goodbye && sleep 3"
    output_type: aws
    aws:
      log_group: test-thing
      log_stream_prefix: goodbye
      region: us-west-2
```

Then pass the configuration file as the only argument to the binary: `monout config.yml`

This will output to the log group named `test-thing`, to a stream "YYYY/mm/dd/{log_streem_prefix}".  

You can run this if you have the AWS environment variables set (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, etc.), or
have granted the necessary permissions to the EC2/ECS instance running. 

### Options

`output_type` can be either `aws` or `console`. If you specify `console` you can omit the AWS options. There are 
currently no configurable options for `console`. 