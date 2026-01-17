use vrp::solver::tabu_search::search;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    search::run().await
}
