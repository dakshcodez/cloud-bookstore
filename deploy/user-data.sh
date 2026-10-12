#!/bin/bash
# EC2 launch template user-data (Ubuntu Server 24.04 LTS).
# Edit the three variables below, then paste the script into the launch template.
set -euxo pipefail

S3_BUCKET="CHANGE-ME-bucket"          # bucket holding bookstore-release.tar.gz
DATABASE_URL="postgres://bookstore:CHANGE-ME@CHANGE-ME.rds.amazonaws.com:5432/bookstore"
PORT=8080

# Instance metadata (IMDSv2) so the UI can show which instance served the request.
TOKEN=$(curl -s -X PUT http://169.254.169.254/latest/api/token -H "X-aws-ec2-metadata-token-ttl-seconds: 300")
IMDS() { curl -s -H "X-aws-ec2-metadata-token: $TOKEN" "http://169.254.169.254/latest/meta-data/$1"; }
INSTANCE_ID=$(IMDS instance-id)
AVAILABILITY_ZONE=$(IMDS placement/availability-zone)

# Ubuntu doesn't ship the AWS CLI; install v2 (needs outbound internet via the NAT gateway).
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y curl unzip
curl -s "https://awscli.amazonaws.com/awscli-exe-linux-$(uname -m).zip" -o /tmp/awscliv2.zip
unzip -q /tmp/awscliv2.zip -d /tmp
/tmp/aws/install

mkdir -p /opt/bookstore
aws s3 cp "s3://${S3_BUCKET}/bookstore-release.tar.gz" /tmp/release.tar.gz
tar -xzf /tmp/release.tar.gz -C /opt/bookstore   # contains: bookstore, frontend/

cat > /etc/bookstore.env <<ENV
DATABASE_URL=${DATABASE_URL}
PORT=${PORT}
FRONTEND_DIR=/opt/bookstore/frontend
INSTANCE_ID=${INSTANCE_ID}
AVAILABILITY_ZONE=${AVAILABILITY_ZONE}
RUST_LOG=info
ENV
chmod 600 /etc/bookstore.env

cat > /etc/systemd/system/bookstore.service <<UNIT
[Unit]
Description=Bookstore (Rust)
After=network-online.target

[Service]
EnvironmentFile=/etc/bookstore.env
ExecStart=/opt/bookstore/bookstore
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
UNIT

chmod +x /opt/bookstore/bookstore
systemctl daemon-reload
systemctl enable --now bookstore
