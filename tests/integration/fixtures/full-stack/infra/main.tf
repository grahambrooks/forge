resource "aws_ecs_service" "api" {
  name            = "fullstack-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = 2
}

resource "aws_rds_instance" "db" {
  identifier     = "fullstack-db"
  engine         = "postgres"
  instance_class = "db.t3.micro"
}
