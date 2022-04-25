@Library('jenkins-library')

def pipeline = new org.docker.AppPipeline(steps: this,
    dockerImageName:              'iroha2-block-explorer-backend',
    dockerRegistryCred:           'bot-iroha2-rw',
    dockerFileName:               "${env.GIT_BRANCH=='master'?'release':'debug'}/Dockerfile",
    triggerCommonBuildExpression: (env.BRANCH_NAME in ['develop','master', 'feature/DOPS-1749/create-CI']),
    secretScannerExclusion:       '.*Cargo.toml',
    nameCI:                       'iroha2-block-explorer-backend-CI')
pipeline.runPipeline()