# monout

A utility to monitor and direct the output of running services to external logging tools. 

## Project Status

This is currently an experiment. Support for outputting to the console and to AWS Cloudwatch are baked in. If I continue
with this project I will make it more easily extensible to pipe logs and metrics anywhere you like. The configuration 
file format will be considered stable once this project reaches version 1. 

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

This will output to the log group named `test-thing`, to a stream "YYYY/mm/dd/{log_streem_prefix}".  

You can run this if you have the AWS environment variables set (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, etc.), or
have granted the necessary permissions to the EC2/ECS instance running. 

### Options

`output_type` can be either `aws` or `console`. If you specify `console` you can omit the AWS options. There are 
currently no configurable options for `console`. 