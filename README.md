# Online Book Store: 3-tier app on AWS

Project for my cloud architecture class. A deliberately small book store (plain HTML/JS frontend, Rust backend, PostgreSQL) used to demonstrate a 3-tier AWS architecture: VPC, Security Groups, EC2, EBS, ALB, Auto Scaling, RDS and AWS Budgets.

## Architecture

```
Internet -> ALB (public subnets, 2 AZs)
              -> Auto Scaling Group of EC2 running the Rust app (private subnets)
                    -> RDS PostgreSQL (private subnets)
```

| Component | Setup |
|---|---|
| VPC | 2 public subnets (ALB, NAT gateway), 2 private subnets (EC2, RDS), across 2 AZs |
| Security groups | ALB: 80 from anywhere. EC2: 8080 from ALB SG only. RDS: 5432 from EC2 SG only |
| EC2 + EBS | Launch template, Amazon Linux 2023, gp3 root volume, `deploy/user-data.sh` |
| Load balancer | Application Load Balancer, target group health check `GET /health` |
| Auto Scaling | min 2, max 4, target tracking on average CPU ~50% |
| RDS | PostgreSQL, db.t3.micro, private subnets |
| Budgets | Monthly cost budget with email alert |

## Application

- `frontend/`: static HTML/CSS/JS, served by the Rust server.
- `backend/`: Rust (axum + sqlx). Creates and seeds its tables on startup.

| Endpoint | Purpose |
|---|---|
| `GET /health` | ALB health check (checks DB connectivity) |
| `GET /api/info` | Instance ID and AZ, shown in the UI header to demonstrate load balancing |
| `GET /api/books` | List books |
| `GET/POST /api/orders` | List / place orders (`{"book_id": 1, "customer_name": "..."}`) |
| `GET /api/stress?ms=200` | Burns CPU for up to 1000 ms, used to trigger Auto Scaling |

Environment variables: `DATABASE_URL` (required), `PORT` (default 8080), `FRONTEND_DIR` (default `frontend`), `INSTANCE_ID`, `AVAILABILITY_ZONE`.

## Run locally

```bash
docker compose up -d                      # local Postgres
cd backend
DATABASE_URL=postgres://bookstore:bookstore@localhost:5432/bookstore \
FRONTEND_DIR=../frontend cargo run
# open http://localhost:8080
```

## Deploy to AWS (Learner Lab)

The Learner Lab environment can't be saved, so the app is packaged to be redeployed quickly each session.

1. **Build** a static binary and package it with the frontend (needs Docker):
   `./deploy/build.sh` -> `dist/bookstore-release.tar.gz`
2. **Upload** it to an S3 bucket: `aws s3 cp dist/bookstore-release.tar.gz s3://<bucket>/`
3. **Network**: create the VPC, subnets, route tables, IGW and NAT gateway, then the three security groups.
4. **RDS**: create PostgreSQL in the private subnets with database name `bookstore`.
5. **Launch template**: Amazon Linux 2023, IAM instance profile `LabInstanceProfile`, EC2 security group, and `deploy/user-data.sh` as user data (edit the bucket and `DATABASE_URL` first).
6. **ALB + target group**: HTTP:80 -> instances on 8080, health check path `/health`.
7. **Auto Scaling group**: private subnets, attach to the target group, use ELB health checks, add a target tracking policy (average CPU 50%).
8. **Budget**: create a monthly cost budget with an email alert.

## Load test (Auto Scaling)

```bash
k6 run -e BASE_URL=http://<alb-dns-name> loadtest/k6.js
```

The script ramps up to 100 virtual users over about 4 minutes, holds for 5, then ramps down. About 30% of requests hit `/api/stress`, which is what pushes CPU over the scaling target. Watch the ASG activity history and the CloudWatch CPU graph for the scale-out and scale-in events.
