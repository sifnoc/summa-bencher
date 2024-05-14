import os
import subprocess
import uuid
import boto3
from botocore.exceptions import NoCredentialsError

def get_instance_id_or_uuid(timeout_seconds=3):
    try:
        # Run the ec2metadata command with a timeout
        result = subprocess.run(["ecmetadata", "--instance-id"], check=True, text=True, stdout=subprocess.PIPE, timeout=timeout_seconds)
        instance_id = result.stdout.strip()
        if instance_id:
            return instance_id[2:] # Strip the 'i-' prefix
    except subprocess.TimeoutExpired:
        print("The ec2metadata command timed out, It seems not to be running on an EC2 instance.")
    except subprocess.CalledProcessError:
        print("Failed to retrieve instance ID.")
    except FileNotFoundError:
        print("ec2metadata command not found.")

    print("use uuid instead of instance id")
    # Fallback to generating a UUID if ec2metadata fails or times out
    return str(uuid.uuid4().node)


# Benchmark
try:
  # K is only used in V1. otherwise K is replaced by `LEVELS`
  k = os.environ['K']
  n_levels = os.environ['N_LEVELS'] # Used as K in v2 and v3
  n_currencies = os.environ['N_CURRENCIES']
  region_name = os.environ['REGION_NAME']
  bucket_name = os.environ['S3_BUCKET']
except KeyError as e:
    print(f"Environment variable {str(e)} not found")
    exit(1)
    
print(f"Running benchmarks with {n_levels} levels and {n_currencies} currencies")

try:
    # Run benchmarks
    bench_v1 = subprocess.run(['cargo', 'bench', '--bench', 'v1'], capture_output=True)
    print("run result in v1:", bench_v1.stdout.decode())

    bench_v2 = subprocess.run(['cargo', 'bench', '--bench', 'v2'], capture_output=True)
    print("run result in v2:", bench_v2.stdout.decode())

    bench_v3 = subprocess.run(['cargo', 'bench', '--bench', 'v3'], capture_output=True)
    print("run result in v3:", bench_v3.stdout.decode())
except Exception as e:
    print(f"An error occurred: {str(e)}")
    exit(1)

# Load environment variables
aws_access_key_id = os.environ['AWS_ACCESS_KEY_ID']
aws_secret_access_key = os.environ['AWS_SECRET_ACCESS_KEY']
aws_session_token = os.environ['AWS_SESSION_TOKEN']

# Initialize AWS S3 Client
session = boto3.Session(
    aws_access_key_id=aws_access_key_id,
    aws_secret_access_key=aws_secret_access_key,
    aws_session_token=aws_session_token,
    region_name=region_name,
)
s3 = session.client('s3')

try:
  # Check if the bucket exists and you have permission to access it
  s3.head_bucket(Bucket=bucket_name)
  print("Successfully connected to the bucket.")
except boto3.exceptions.botocore.exceptions.ClientError as e:
    # Handle the error including bucket not existing or forbidden access
    error_code = int(e.response['Error']['Code'])
    if error_code == 404:
        print(f'"{bucket_name}" bucketdoes not exist.')
        exit(1)
    elif error_code == 403:
        print("Access to bucket forbidden.")
        exit(1)
    else:
        print(f"An error occurred: {e}")
        exit(1)

# Upload benchmark results to S3
benchmark_id = get_instance_id_or_uuid()
n_users = 1 << int(n_levels)
bench_result_v1_filename = f'v1_k{k}_u{n_users}_c{n_currencies}.json'
bench_result_v2_filename = f'v2_k{n_levels}_u{n_users - 6}_c{n_currencies}.json'
bench_result_v3_filename = f'v3_k{n_levels}_u{n_users - 6}_c{n_currencies}.json'

for filename in [bench_result_v1_filename, bench_result_v2_filename, bench_result_v3_filename]:
  try:
    s3.upload_file(filename, bucket_name, f'{filename.replace(".", f"_{benchmark_id}.")}', ExtraArgs={'ACL': 'public-read'})
  except FileNotFoundError:
      print("The result file was not found")
      exit(1)
  except NoCredentialsError:
      print("Credentials not available")
      exit(1)
  
exit(0)
