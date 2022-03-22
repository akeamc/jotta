# `jotta-rest`

A RESTful adapter to Jottacloud.

## API reference

The API is very similar to the [Google Cloud Storage JSON API](https://cloud.google.com/storage/docs/json_api).

Every file is represented as an object, and objects are in turn stored in buckets.

### Buckets

#### Listing buckets

```
GET /b
```

#### Getting a bucket

```
GET /b/{bucket}
```

### Objects

#### Listing objects in a bucket

```
GET /b/{bucket}/o
```

#### Getting an object

```
GET /b/{bucket}/o/{object}
```

<table class="matchpre" id="request_parameters">
  <thead>
    <tr>
      <th>Parameter name</th>
      <th>Type</th>
      <th>Description</th>
    </tr>
  </thead>
  <tbody>
  <tr>
    <td colspan="3"><b>Optional query parameters</b></td>
  </tr>
    <tr>
      <td><code>alt</code></td>
      <td><code>string</code></td>
      <td>
        What type of data to return. May be:
        <ul>
          <li><code>json</code> (default): Return object metadata.</li>
          <li><code>media</code>: Return object data.</li>
        </ul>
      </td>
    </tr>
  </tbody>
</table>
