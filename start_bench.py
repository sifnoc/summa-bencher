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
  levels = os.environ['LEVELS'] # Used as K in V3s
  n_currencies = os.environ['N_CURRENCIES']
  region_name = os.environ['REGION_NAME']
  bucket_name = os.environ['S3_BUCKET']
except KeyError as e:
    print(f"Environment variable {str(e)} not found")
    exit(1)
    
print(f"Running benchmarks with {levels} levels and {n_currencies} currencies")

try:
    # Run benchmarks
    bench_v3a = subprocess.run(['cargo', 'bench', '--bench', 'v3a'], capture_output=True)
    print("run result in v3a:", bench_v3a.stdout.decode())

    bench_v3b = subprocess.run(['cargo', 'bench', '--bench', 'v3b'], capture_output=True)
    print("run result in v3b:", bench_v3b.stdout.decode())

    bench_v3c = subprocess.run(['cargo', 'bench', '--bench', 'v3c'], capture_output=True)
    print("run result in v3c:", bench_v3c.stdout.decode())
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
n_users = 1 << int(levels)
bench_result_v3a_filename = f'v3a_k{levels}_u{n_users - 1}_c{n_currencies}.json'
bench_result_v3b_filename = f'v3b_k{levels}_u{n_users - 2}_c3.json'
bench_result_v3c_filename = f'v3c_k{levels}_u{n_users - 2}_c1.json'

for filename in [bench_result_v3a_filename, bench_result_v3b_filename, bench_result_v3c_filename]:
  try:
    s3.upload_file(filename, bucket_name, f'{filename.replace(".", f"_{benchmark_id}.")}', ExtraArgs={'ACL': 'public-read'})
  except FileNotFoundError:
      print("The result file was not found")
      exit(1)
  except NoCredentialsError:
      print("Credentials not available")
      exit(1)
  
exit(0)
