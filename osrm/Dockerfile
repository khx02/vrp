FROM ghcr.io/project-osrm/osrm-backend:latest

WORKDIR /data

# Download map data
RUN wget --continue --tries=10 --timeout=60 \
    https://download.geofabrik.de/asia/malaysia-singapore-brunei-260118.osm.pbf \
    -O /data/map.osm.pbf

# Process the map data in separate steps to manage memory
RUN osrm-extract -p /opt/car.lua /data/map.osm.pbf

RUN osrm-partition /data/map.osrm

RUN osrm-customize /data/map.osrm

# Clean up raw data
RUN rm /data/map.osm.pbf

EXPOSE 5000

CMD ["osrm-routed", "--algorithm", "mld", "/data/map.osrm"]
