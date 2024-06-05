#!/usr/bin/env bash
set -e

printf "SEMVER = ${SEMVER}\n"
printf "VERSION = ${VERSION}\n"
printf "ARCH = ${ARCH}\n"
printf "PLATFORM = ${PLATFORM}\n"

rm -rf ./build/dir_zip
rm -rf ./build/pg_bestmatch-pg${VERSION}_${ARCH}-unknown-linux-gnu_${SEMVER}.zip
rm -rf ./build/dir_deb
rm -rf ./build/pg_bestmatch-pg${VERSION}_${SEMVER}_${PLATFORM}.deb

mkdir -p ./build/dir_zip
cp -a ./sql/upgrade/. ./build/dir_zip/
cp ./target/pg_bestmatch--$SEMVER.sql ./build/dir_zip/pg_bestmatch--$SEMVER.sql
sed -e "s/@CARGO_VERSION@/$SEMVER/g" < ./pg_bestmatch.control > ./build/dir_zip/pg_bestmatch.control
cp ./target/${ARCH}-unknown-linux-gnu/release/libpg_bestmatch.so ./build/dir_zip/pg_bestmatch.so
zip ./build/pg_bestmatch-pg${VERSION}_${ARCH}-unknown-linux-gnu_${SEMVER}.zip -j ./build/dir_zip/*

mkdir -p ./build/dir_deb
mkdir -p ./build/dir_deb/DEBIAN
mkdir -p ./build/dir_deb/usr/share/postgresql/$VERSION/extension/
mkdir -p ./build/dir_deb/usr/lib/postgresql/$VERSION/lib/
for file in $(ls ./build/dir_zip/*.sql | xargs -n 1 basename); do
    cp ./build/dir_zip/$file ./build/dir_deb/usr/share/postgresql/$VERSION/extension/$file
done
for file in $(ls ./build/dir_zip/*.control | xargs -n 1 basename); do
    cp ./build/dir_zip/$file ./build/dir_deb/usr/share/postgresql/$VERSION/extension/$file
done
for file in $(ls ./build/dir_zip/*.so | xargs -n 1 basename); do
    cp ./build/dir_zip/$file ./build/dir_deb/usr/lib/postgresql/$VERSION/lib/$file
done
echo "Package: postgresql-${VERSION}-pg-bestmatch
Version: ${SEMVER}
Section: database
Priority: optional
Architecture: ${PLATFORM}
Maintainer: Tensorchord <support@tensorchord.ai>
Description: Generate BM25 sparse vector inside PostgreSQL
Homepage: https://github.com/tensorchord/pg_bestmatch.rs
License: apache2" \
> ./build/dir_deb/DEBIAN/control
(cd ./build/dir_deb && md5sum usr/share/postgresql/$VERSION/extension/* usr/lib/postgresql/$VERSION/lib/*) > ./build/dir_deb/DEBIAN/md5sums
dpkg --build ./build/dir_deb/ ./build/postgresql-${VERSION}-pg-bestmatch_${SEMVER}_${PLATFORM}.deb
