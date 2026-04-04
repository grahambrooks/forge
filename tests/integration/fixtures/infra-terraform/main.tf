provider "aws" {
  region = "us-east-1"
}

resource "aws_lambda_function" "api" {
  function_name = "payment-api"
  runtime       = "provided.al2023"
  handler       = "bootstrap"
  memory_size   = 256
  timeout       = 30
}

resource "aws_dynamodb_table" "orders" {
  name         = "orders"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "id"

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_sqs_queue" "events" {
  name                       = "payment-events"
  message_retention_seconds  = 86400
  visibility_timeout_seconds = 60
}
