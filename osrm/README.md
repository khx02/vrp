Run the following commands to setup osrm backend:

```dockerfile
docker build -t osrm .

docker run -d \
  --name osrm-backend \
  -p 6000:5000 \
  -v "$(pwd)/osrm-data:/data" \
  --restart unless-stopped \
  osrm
```
