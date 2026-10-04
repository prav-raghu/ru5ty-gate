project_id   = "ru5ty-gate-prod"
region       = "africa-south1"
environment  = "prod"
project_name = "ru5ty-gate"

customer_api_image = "africa-south1-docker.pkg.dev/ru5ty-gate-prod/ru5ty-gate-prod/customer-api:latest"
admin_api_image    = "africa-south1-docker.pkg.dev/ru5ty-gate-prod/ru5ty-gate-prod/admin-api:latest"
customer_web_image = "africa-south1-docker.pkg.dev/ru5ty-gate-prod/ru5ty-gate-prod/customer-web:latest"

customer_api_min_instances = 1
customer_api_max_instances = 20
admin_api_min_instances    = 1
admin_api_max_instances    = 10
customer_web_min_instances = 1
customer_web_max_instances = 20

redis_memory_size_gb = 2
redis_tier           = "STANDARD_HA"

customer_web_url = "https://app.example.com"
admin_web_url    = "https://admin.example.com"
admin_web_domain = "admin.example.com"
