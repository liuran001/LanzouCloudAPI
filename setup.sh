#!/usr/bin/env bash

for rq in curl tar grep sed systemctl python3; do
    [ "$(command -v $rq)" ] || {
        echo "Lack of $rq, quit installation"
        exit
    }
done

REPO=LanzouCloudAPI
DOMAIN=pan.lanzou.com
IP=47.91.203.9

curl -sSL https://raw.githubusercontent.com/python-poetry/poetry/master/get-poetry.py | python3 -

cd /usr/local/share
rm -f master.tar.gz
curl -LO "https://github.com/vcheckzen/$REPO/archive/master.tar.gz"

rm -rf "$REPO"
mkdir "$REPO"
tar xf master.tar.gz -C "$REPO" --strip-components 1
rm -f master.tar.gz

cd "$REPO"
poetry config virtualenvs.in-project true
poetry install

grep "$DOMAIN" /etc/hosts &>/dev/null && {
    sed -i "s/.*$DOMAIN/$IP $DOMAIN/" /etc/hosts
} || {
    echo "$IP $DOMAIN" >>/etc/hosts
}

systemctl stop lanzous 2>/dev/null

cp lanzous.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now lanzous
