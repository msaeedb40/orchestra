package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/orchestra/orchestra/api/v1alpha1"
)

// ClientConfig holds configuration for the Orchestra API client.
type ClientConfig struct {
	BaseURL    string
	HTTPClient *http.Client
	Token      string
}

// Client is an HTTP client for the Orchestra API server.
type Client struct {
	cfg ClientConfig
}

// NewClient returns a new Client. If cfg.HTTPClient is nil, a default client
// with a 30-second timeout is used.
func NewClient(cfg ClientConfig) *Client {
	if cfg.HTTPClient == nil {
		cfg.HTTPClient = &http.Client{Timeout: 30 * time.Second}
	}
	return &Client{cfg: cfg}
}

// do builds, executes, and validates an HTTP request. Non-2xx responses are
// returned as *APIError. The caller is responsible for closing the response body.
func (c *Client) do(ctx context.Context, method, url string, body io.Reader) (*http.Response, error) {
	req, err := http.NewRequestWithContext(ctx, method, url, body)
	if err != nil {
		return nil, fmt.Errorf("building request: %w", err)
	}

	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	if c.cfg.Token != "" {
		req.Header.Set("Authorization", "Bearer "+c.cfg.Token)
	}

	resp, err := c.cfg.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("executing request: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		defer resp.Body.Close()
		rawBody, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		msg := string(rawBody)
		if len(msg) > 256 {
			msg = msg[:256]
		}
		return nil, &APIError{
			StatusCode: resp.StatusCode,
			Message:    msg,
			Body:       string(rawBody),
		}
	}

	return resp, nil
}

// decode JSON-decodes the response body into v and closes it.
func decode(resp *http.Response, v interface{}) error {
	defer resp.Body.Close()
	if err := json.NewDecoder(resp.Body).Decode(v); err != nil {
		return fmt.Errorf("decoding response: %w", err)
	}
	return nil
}

// ListClusters returns all OrchestraClusters from the API server.
func (c *Client) ListClusters(ctx context.Context) ([]v1alpha1.OrchestraCluster, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters"
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var list v1alpha1.OrchestraClusterList
	if err := decode(resp, &list); err != nil {
		return nil, err
	}
	return list.Items, nil
}

// GetCluster returns the named OrchestraCluster.
func (c *Client) GetCluster(ctx context.Context, name string) (*v1alpha1.OrchestraCluster, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters/" + name
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var cluster v1alpha1.OrchestraCluster
	if err := decode(resp, &cluster); err != nil {
		return nil, err
	}
	return &cluster, nil
}

// CreateCluster creates a new OrchestraCluster and returns the server's response.
func (c *Client) CreateCluster(ctx context.Context, cluster *v1alpha1.OrchestraCluster) (*v1alpha1.OrchestraCluster, error) {
	b, err := json.Marshal(cluster)
	if err != nil {
		return nil, fmt.Errorf("marshalling cluster: %w", err)
	}
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters"
	resp, err := c.do(ctx, http.MethodPost, url, bytes.NewBuffer(b))
	if err != nil {
		return nil, err
	}
	var created v1alpha1.OrchestraCluster
	if err := decode(resp, &created); err != nil {
		return nil, err
	}
	return &created, nil
}

// DeleteCluster deletes the named OrchestraCluster.
func (c *Client) DeleteCluster(ctx context.Context, name string) error {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters/" + name
	resp, err := c.do(ctx, http.MethodDelete, url, nil)
	if err != nil {
		return err
	}
	resp.Body.Close()
	return nil
}

// ListNodes returns all OrchestraNodes belonging to clusterName.
func (c *Client) ListNodes(ctx context.Context, clusterName string) ([]v1alpha1.OrchestraNode, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters/" + clusterName + "/nodes"
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var list v1alpha1.OrchestraNodeList
	if err := decode(resp, &list); err != nil {
		return nil, err
	}
	return list.Items, nil
}

// GetNode returns a specific OrchestraNode within a cluster.
func (c *Client) GetNode(ctx context.Context, clusterName, nodeName string) (*v1alpha1.OrchestraNode, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/clusters/" + clusterName + "/nodes/" + nodeName
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var node v1alpha1.OrchestraNode
	if err := decode(resp, &node); err != nil {
		return nil, err
	}
	return &node, nil
}

// ListNetworks returns all OrchestraNetworks.
func (c *Client) ListNetworks(ctx context.Context) ([]v1alpha1.OrchestraNetwork, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/networks"
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var list v1alpha1.OrchestraNetworkList
	if err := decode(resp, &list); err != nil {
		return nil, err
	}
	return list.Items, nil
}

// GetNetwork returns the OrchestraNetwork associated with clusterName.
func (c *Client) GetNetwork(ctx context.Context, clusterName string) (*v1alpha1.OrchestraNetwork, error) {
	url := c.cfg.BaseURL + "/apis/orchestra.io/v1alpha1/networks/" + clusterName
	resp, err := c.do(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	var network v1alpha1.OrchestraNetwork
	if err := decode(resp, &network); err != nil {
		return nil, err
	}
	return &network, nil
}
