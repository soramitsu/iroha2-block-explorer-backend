@Library('jenkins-library')

def pipeline = new org.docker.AppPipeline(steps: this,
    dockerImageName:        'iroha2/iroha2-block-explorer-backend',
    dockerRegistryCred:     'bot-iroha2-rw',
    dockerImageTags:        ['origin/feature/DOPS-1749/create-CI': 'dev', 'feature/DOPS-1749/create-CI': 'dev', 'master': 'latest', 'origin/master': 'latest'],
    secretScannerExclusion: '.*Cargo.toml')
pipeline.runPipeline()