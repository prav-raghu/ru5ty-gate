terraform {
  required_version = ">= 1.7.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  backend "s3" {
    bucket         = "ru5ty-gate-tfstate"
    key            = "terraform/state/aws.tfstate"
    region         = "af-south-1"
    encrypt        = true
    dynamodb_table = "ru5ty-gate-tfstate-lock"
  }
}

provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Project     = var.project_name
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}
