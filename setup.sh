#!/usr/bin/env bash

REPO=LanzouCloudAPI

curl -sSL https://raw.githubusercontent.com/python-poetry/poetry/master/get-poetry.py | python3 -

cd /usr/local/share
curl -LO "https://github.com/vcheckzen/$REPO/archive/master.tar.gz"

mkdir "$REPO"
tar xf master.tar.gz -C "$REPO" --strip-components 1

cd "$REPO"
poetry config virtualenvs.in-project true
poetry install

echo '47.91.203.9 pan.lanzou.com' >>/etc/hosts

cp lanzous.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now lanzous
