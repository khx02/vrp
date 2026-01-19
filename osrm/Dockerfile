FROM ghcr.io/project-osrm/osrm-backend:latest

WORKDIR /data

RUN wget --continue --tries=10 --timeout=60 \
     https://download.geofabrik.de/asia/malaysia-singapore-brunei-260118.osm.pbf \
     -O map.osm.pbf

RUN osrm-extract -p /opt/car.lua /data/map.osm.pbf && \
    osrm-partition /data/map.osrm && \
    osrm-customize /data/map.osrm

EXPOSE 5000

CMD ["osrm-routed", "--algorithm", "mld", "/data/map.osrm"]
